const { ethers } = require('hardhat');
const {
    Wallet,
    Contract,
    getDefaultProvider,
    utils: { defaultAbiCoder, isAddress },
} = ethers;

const {
    deployContract,
    printWalletInfo,
    saveConfig,
    printInfo,
    printWarn,
    printError,
    mainProcessor,
    prompt,
    sleep,
    getBytecodeHash,
    getGasOptions,
    isContract,
    isValidAddress,
    getDeployOptions,
    getDeployedAddress,
    wasEventEmitted,
} = require('../evm/utils');

const { getContractJSONWithHTS } = require('./utils.js');
const { printHederaNetwork, addBaseOptions } = require('./cli-utils.js');

const { addEvmOptions } = require('../evm/cli-utils');
const { Command, Option } = require('commander');
const { WHBAR_ABI, fundWithWHBAR } = require('./fund-whbar.js');
const { DEFAULT_TOKEN_CREATION_PRICE_TINY_CENTS, TINY_PARTS_PER_WHOLE } = require('./token-creation-price.js');

/**
 * Function that handles the ITS deployment.
 * @param {*} wallet
 * @param {*} chain
 * @param {*} deployOptions
 * @param {*} operatorAddress
 * @param {*} skipExisting
 * @param {*} verifyOptions
 */

async function deployAll(config, wallet, chain, options) {
    const { env, artifactPath, deployMethod, proxyDeployMethod, skipExisting, verify, yes, predictOnly } = options;
    const verifyOptions = verify ? { env, chain: chain.name, only: verify === 'only' } : null;

    const getContractJSON = getContractJSONWithHTS(options.htsLibraryAddress);

    const provider = getDefaultProvider(chain.rpc);
    const InterchainTokenService = getContractJSON('InterchainTokenService', artifactPath);

    const contractName = 'InterchainTokenService';
    const itsFactoryContractName = 'InterchainTokenFactory';
    const contracts = chain.contracts;

    const contractConfig = contracts[contractName] || {};
    const itsFactoryContractConfig = contracts[itsFactoryContractName] || {};

    const salt = options.salt ? `ITS ${options.salt}` : 'ITS';
    let proxySalt, factorySalt;

    // If reusing the proxy, then proxy salt is the existing value
    if (options.reuseProxy) {
        proxySalt = contractConfig.proxySalt;
        factorySalt = itsFactoryContractConfig.salt;
    } else if (options.proxySalt) {
        proxySalt = `ITS ${options.proxySalt}`;
        factorySalt = `ITS Factory ${options.proxySalt}`;
    } else if (options.salt) {
        proxySalt = `ITS ${options.salt}`;
        factorySalt = `ITS Factory ${options.salt}`;
    } else {
        proxySalt = 'ITS';
        factorySalt = 'ITS Factory';
    }

    const implementationSalt = `${salt} Implementation`;

    contractConfig.salt = salt;
    contractConfig.proxySalt = proxySalt;
    contractConfig.deployer = wallet.address;

    itsFactoryContractConfig.deployer = wallet.address;
    itsFactoryContractConfig.salt = factorySalt;

    contracts[contractName] = contractConfig;
    contracts[itsFactoryContractName] = itsFactoryContractConfig;

    const proxyJSON = getContractJSON('InterchainProxy', artifactPath);
    const predeployCodehash = await getBytecodeHash(proxyJSON, chain.axelarId);
    const gasOptions = await getGasOptions(chain, options, contractName);
    const deployOptions = getDeployOptions(deployMethod, salt, chain);

    const interchainTokenService = options.reuseProxy
        ? contractConfig.address
        : await getDeployedAddress(wallet.address, proxyDeployMethod, {
              salt: proxySalt,
              deployerContract: getDeployOptions(proxyDeployMethod, proxySalt, chain).deployerContract,
              contractJson: proxyJSON,
              constructorArgs: [],
              provider: wallet.provider,
          });

    if (!isValidAddress(interchainTokenService)) {
        throw new Error(`Invalid ITS address: ${interchainTokenService}`);
    }

    if (options.reuseProxy) {
        printInfo('Reusing existing Interchain Token Service proxy', interchainTokenService);
    } else {
        printInfo('Interchain Token Service will be deployed to', interchainTokenService);
    }

    const interchainTokenFactory = options.reuseProxy
        ? itsFactoryContractConfig.address
        : await getDeployedAddress(wallet.address, proxyDeployMethod, {
              salt: factorySalt,
              deployerContract: getDeployOptions(proxyDeployMethod, factorySalt, chain).deployerContract,
              contractJSON: proxyJSON,
              constructorArgs: [],
              provider: wallet.provider,
          });

    if (!isValidAddress(interchainTokenFactory)) {
        throw new Error(`Invalid Interchain Token Factory address: ${interchainTokenFactory}`);
    }

    if (options.reuseProxy) {
        printInfo('Reusing existing Interchain Token Factory proxy', interchainTokenFactory);
    } else {
        printInfo('Interchain Token Factory will be deployed to', interchainTokenFactory);
    }

    // Register all EVM chains that ITS is or will be deployed on.
    const itsChains = Object.values(config.chains).filter(
        (chain) => chain.chainType === 'evm' && chain.contracts?.InterchainTokenService?.address,
    );
    const trustedChains = itsChains.map((chain) => chain.axelarId);

    // If ITS Hub is deployed, register it as a trusted chain as well
    const itsHubAddress = config.axelar?.contracts?.InterchainTokenService?.address;

    if (itsHubAddress) {
        if (!config.axelar?.axelarId) {
            throw new Error('Axelar ID for Axelar chain is not set');
        }

        trustedChains.push(config.axelar?.axelarId);
    }

    // Trusted addresses are only used when deploying a new proxy
    if (!options.reuseProxy) {
        printInfo('Trusted chains', trustedChains);
    }

    if (!isAddress(options.whbarAddress)) {
        throw new Error(`Invalid WHBAR address: ${options.whbarAddress}`);
    }

    const existingAddress = config.chains.ethereum?.contracts?.[contractName]?.address;

    if (existingAddress !== undefined && interchainTokenService !== existingAddress) {
        printWarn(
            `Predicted address ${interchainTokenService} does not match existing deployment ${existingAddress} on chain ${config.chains.ethereum.name}`,
        );

        const existingCodeHash = config.chains.ethereum.contracts[contractName].predeployCodehash;

        if (predeployCodehash !== existingCodeHash) {
            printWarn(
                `Pre-deploy bytecode hash ${predeployCodehash} does not match existing deployment's predeployCodehash ${existingCodeHash} on chain ${config.chains.ethereum.name}`,
            );
        }

        printWarn('For official deployment, recheck the deployer, salt, args, or contract bytecode');
    }

    if (predictOnly || prompt(`Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    const deployments = {
        tokenManagerDeployer: {
            name: 'Token Manager Deployer',
            contractName: 'TokenManagerDeployer',
            async deploy() {
                return deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('TokenManagerDeployer', artifactPath),
                    [],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        interchainTokenDeployer: {
            name: 'Interchain Token Deployer',
            contractName: 'InterchainTokenDeployer',
            async deploy() {
                return deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('InterchainTokenDeployer', artifactPath),
                    [],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        tokenManager: {
            name: 'Token Manager',
            contractName: 'TokenManager',
            async deploy() {
                return deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('TokenManager', artifactPath),
                    [interchainTokenService],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        tokenHandler: {
            name: 'Token Handler',
            contractName: 'TokenHandler',
            async deploy() {
                return deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('TokenHandler', artifactPath),
                    [],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        implementation: {
            name: 'Interchain Token Service Implementation',
            contractName: 'InterchainTokenService',
            async deploy() {
                const args = [
                    contractConfig.tokenManagerDeployer,
                    contractConfig.interchainTokenDeployer,
                    contracts.AxelarGateway.address,
                    contracts.AxelarGasService.address,
                    interchainTokenFactory,
                    chain.axelarId,
                    itsHubAddress,
                    contractConfig.tokenManager,
                    contractConfig.tokenHandler,
                    options.whbarAddress,
                ];

                printInfo('ITS Implementation args', args);

                return deployContract(
                    proxyDeployMethod,
                    wallet,
                    InterchainTokenService,
                    args,
                    getDeployOptions(proxyDeployMethod, implementationSalt, chain),
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        address: {
            name: 'Interchain Token Service Proxy',
            contractName: 'InterchainProxy',
            async deploy() {
                const operatorAddress = options.operatorAddress || wallet.address;

                const tokenCreationPrice = options.tokenCreationPrice || DEFAULT_TOKEN_CREATION_PRICE_TINY_CENTS;

                const deploymentParams = defaultAbiCoder.encode(
                    ['address', 'string', 'string[]', 'uint256'],
                    [operatorAddress, chain.axelarId, trustedChains, tokenCreationPrice],
                );
                contractConfig.predeployCodehash = predeployCodehash;

                printInfo(
                    `Setting ITS token creation price to ${tokenCreationPrice} tinycents ($${tokenCreationPrice / TINY_PARTS_PER_WHOLE})`,
                );

                const args = [contractConfig.implementation, wallet.address, deploymentParams];
                printInfo('ITS Proxy args', args);

                return deployContract(
                    proxyDeployMethod,
                    wallet,
                    getContractJSON('InterchainProxy', artifactPath),
                    args,
                    getDeployOptions(proxyDeployMethod, proxySalt, chain),
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        interchainTokenFactoryImplementation: {
            name: 'Interchain Token Factory Implementation',
            contractName: 'InterchainTokenFactory',
            async deploy() {
                return deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('InterchainTokenFactory', artifactPath),
                    [interchainTokenService],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        interchainTokenFactory: {
            name: 'Interchain Token Factory Proxy',
            contractName: 'InterchainProxy',
            async deploy() {
                const args = [itsFactoryContractConfig.implementation, wallet.address, '0x'];
                printInfo('ITS Factory Proxy args', args);

                return deployContract(
                    proxyDeployMethod,
                    wallet,
                    getContractJSON('InterchainProxy', artifactPath),
                    args,
                    getDeployOptions(proxyDeployMethod, factorySalt, chain),
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
    };

    for (const key in deployments) {
        if (skipExisting && contractConfig[key]) continue;

        const deployment = deployments[key];

        // When upgrading/reusing proxy, avoid re-deploying the proxy and the interchain token contract
        if (options.reuseProxy && ['InterchainToken', 'InterchainProxy'].includes(deployment.contractName)) {
            printInfo(`Reusing ${deployment.name} deployment`);
            continue;
        }

        printInfo(`Deploying ${deployment.name}`);

        const contract = await deployment.deploy();

        if (key === 'interchainTokenFactoryImplementation') {
            itsFactoryContractConfig.implementation = contract.address;
        } else if (key === 'interchainTokenFactory') {
            itsFactoryContractConfig.address = contract.address;
        } else {
            contractConfig[key] = contract.address;
        }

        printInfo(`Deployed ${deployment.name} at ${contract.address}`);

        saveConfig(config, options.env);

        if (chain.chainId !== 31337) {
            await sleep(5000);
        }

        if (!(await isContract(contract.address, provider))) {
            throw new Error(`Contract ${deployment.name} at ${contract.address} was not deployed on ${chain.name}`);
        }

        if (deployment.name === 'Interchain Token Service Proxy') {
            if (options.whbarAddress && options.whbarAmount) {
                printInfo(`Funding InterchainProxy at ${contract.address} with WHBAR...`);

                const whbar = new ethers.Contract(options.whbarAddress, WHBAR_ABI, provider);

                const amount = ethers.utils.parseUnits(options.whbarAmount.toString(), 8); // WHBAR has 8 decimals

                await fundWithWHBAR(whbar, contract.address, amount, wallet);
                printInfo(`Successfully funded InterchainProxy with ${options.whbarAmount} WHBAR.`);
            } else {
                printWarn(`Skipping WHBAR funding of InterchainServiceProxy. Please fund manually.`);
            }
        }
    }
}

async function deploy(config, chain, options) {
    const { privateKey, salt } = options;

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    const contractName = 'InterchainTokenService';

    await printWalletInfo(wallet, options);

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    const operatorAddress = options.operatorAddress || wallet.address;

    if (!isAddress(operatorAddress)) {
        throw new Error(`Invalid operator address: ${operatorAddress}`);
    }

    await deployAll(config, wallet, chain, options);
}

async function upgrade(_, chain, options) {
    const { artifactPath, privateKey, predictOnly } = options;

    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet(privateKey, provider);
    const contractName = 'InterchainTokenService';
    const itsFactoryContractName = 'InterchainTokenFactory';

    await printWalletInfo(wallet, options);

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};
    const itsFactoryContractConfig = contracts[itsFactoryContractName] || {};

    contracts[contractName] = contractConfig;
    contracts[itsFactoryContractName] = itsFactoryContractConfig;

    printInfo(`Upgrading Interchain Token Service.`);

    const InterchainTokenService = getContractJSON('InterchainTokenService', artifactPath);
    const gasOptions = await getGasOptions(chain, options, contractName);
    const contract = new Contract(contractConfig.address, InterchainTokenService.abi, wallet);
    const codehash = await getBytecodeHash(contractConfig.implementation, chain.axelarId, provider);

    printInfo(`ITS Proxy`, contract.address);

    const currImplementation = await contract.implementation();
    printInfo(`Current ITS implementation`, currImplementation);
    printInfo(`New ITS implementation`, contractConfig.implementation);

    if (currImplementation === contractConfig.implementation) {
        printWarn(`ITS implementation is already up to date`);
    } else {
        if (predictOnly || prompt(`Proceed with ITS upgrade on ${chain.name}?`, options.yes)) {
            return;
        }

        const receipt = await contract
            .upgrade(contractConfig.implementation, codehash, '0x', gasOptions)
            .then((tx) => tx.wait(chain.confirmations));

        if (!wasEventEmitted(receipt, contract, 'Upgraded')) {
            printError('Upgrade failed');
            return;
        }

        printInfo(`Upgraded Interchain Token Service`);
    }

    const InterchainTokenFactory = getContractJSON('InterchainTokenFactory', artifactPath);
    const itsFactory = new Contract(itsFactoryContractConfig.address, InterchainTokenFactory.abi, wallet);
    const factoryCodehash = await getBytecodeHash(itsFactoryContractConfig.implementation, chain.axelarId, provider);

    printInfo(`ITS Factory Proxy`, itsFactory.address);

    const factoryImplementation = await itsFactory.implementation();
    printInfo(`Current ITS Factory implementation`, factoryImplementation);
    printInfo(`New ITS Factory implementation`, itsFactoryContractConfig.implementation);

    if (factoryImplementation === itsFactoryContractConfig.implementation) {
        printWarn(`ITS Factory implementation is already up to date`);
    } else {
        if (
            options.predictOnly ||
            prompt(
                `Proceed with ITS Factory upgrade to implementation ${itsFactoryContractConfig.implementation} on ${chain.name}?`,
                options.yes,
            )
        ) {
            return;
        }

        const factoryReceipt = await itsFactory
            .upgrade(itsFactoryContractConfig.implementation, factoryCodehash, '0x', gasOptions)
            .then((tx) => tx.wait(chain.confirmations));

        if (!wasEventEmitted(factoryReceipt, itsFactory, 'Upgraded')) {
            printError('Upgrade failed');
            return;
        }

        printInfo(`Upgraded Interchain Token Factory`);
    }
}

async function processCommand(config, chain, options) {
    if (options.upgrade) {
        await upgrade(config, chain, options);
    } else {
        await deploy(config, chain, options);
    }
}

async function main(options) {
    printHederaNetwork(options);

    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-its').description('Deploy interchain token service and interchain token factory');

    program.addOption(
        new Option('-hts, --htsLibraryAddress <htsEvmAddress>', 'EVM address of the HTS library')
            .env('HTS_LIB_ADDRESS')
            .makeOptionMandatory(true),
    );

    program.addOption(
        new Option('-m, --deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create2'),
    );
    program.addOption(
        new Option(
            '--proxyDeployMethod <proxyDeployMethod>',
            'proxy deployment method, overrides normal deployment method (defaults to create3)',
        )
            .choices(['create', 'create3'])
            .default('create3'),
    );

    addEvmOptions(program, { artifactPath: true, skipExisting: true, upgrade: true, predictOnly: true, ignoreChainNames: true });
    addBaseOptions(program);

    program.addOption(new Option('--reuseProxy', 'reuse existing proxy (useful for upgrade deployments'));
    program.addOption(new Option('--contractName <contractName>', 'contract name').default('InterchainTokenService')); // added for consistency
    program.addOption(new Option('-s, --salt <salt>', 'deployment salt to use for ITS deployment').env('SALT'));
    program.addOption(
        new Option(
            '--proxySalt <proxySalt>',
            'deployment salt to use for ITS proxies, this allows deploying latest releases to new chains while deriving the same proxy address',
        )
            .default('v1.0.0')
            .env('PROXY_SALT'),
    );
    program.addOption(
        new Option('-o, --operatorAddress <operatorAddress>', 'address of the ITS operator/rate limiter').env('OPERATOR_ADDRESS'),
    );

    program.addOption(
        new Option('--tokenCreationPrice <priceInTinycents>', 'price of token creation in tinycents').default(
            DEFAULT_TOKEN_CREATION_PRICE_TINY_CENTS,
        ),
    );

    // WHBAR funding options
    program.addOption(
        new Option('--whbarAddress <whbarAddress>', 'address of the WHBAR contract for funding InterchainProxy').env('WHBAR_ADDRESS'),
    );
    program.addOption(
        new Option('--whbarAmount <whbarAmount>', 'amount of HBAR to deposit as WHBAR for InterchainProxy funding')
            .argParser((value) => parseFloat(value))
            .env('WHBAR_AMOUNT'),
    );

    program.addOption(new Option('-n, --chainName <chainName>', 'chain to run the script over').makeOptionMandatory(true).env('CHAIN'));

    program.action(async (options) => {
        // Making an alias because inherited evm code depends on `chainNames` (plural)
        if (options.chainName) {
            options.chainNames = options.chainName;
        }
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS: deploy };
}
