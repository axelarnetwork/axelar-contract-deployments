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
    getContractJSON,
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
} = require('./utils');
const { addExtendedOptions } = require('./cli-utils');
const { Command, Option } = require('commander');

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
    const { env, artifactPath, deployMethod, proxyDeployMethod, skipExisting, verify, yes } = options;
    const verifyOptions = verify ? { env, chain: chain.name, only: verify === 'only' } : null;

    const provider = getDefaultProvider(chain.rpc);
    const InterchainTokenService = getContractJSON('InterchainTokenService', artifactPath);

    const contractName = 'InterchainTokenService';
    const contracts = chain.contracts;

    // Reset config data if it's a fresh deployment
    if (!skipExisting && !options.reuseProxy) {
        contracts[contractName] = {};
    }

    const contractConfig = contracts[contractName] || {};

    const salt = options.salt ? `ITS ${options.salt}` : 'ITS';
    const proxySalt = options.proxySalt || options.salt ? `ITS ${options.proxySalt || options.salt}` : 'ITS';
    const factorySalt = `${proxySalt} Factory`;
    const implementationSalt = `${salt} Implementation`;
    contractConfig.salt = salt;
    contractConfig.proxySalt = proxySalt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    const proxyJSON = getContractJSON('InterchainProxy', artifactPath);
    const predeployCodehash = await getBytecodeHash(proxyJSON, chain.id);
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

    printInfo('Interchain Token Service will be deployed to', interchainTokenService);

    const interchainTokenFactory = options.reuseProxy
        ? contractConfig.interchainTokenFactory
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

    printInfo('Interchain Token Factory will be deployed to', interchainTokenFactory);

    // Register all chains that ITS is or will be deployed on.
    // Add a "skip": true under ITS key in the config if the chain will not have ITS.
    const itsChains = Object.values(config.chains).filter((chain) => chain.contracts?.InterchainTokenService?.skip !== true);
    const trustedChains = itsChains.map((chain) => chain.id);
    const trustedAddresses = itsChains.map((_) => chain.contracts?.InterchainTokenService?.address || interchainTokenService);

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

    if (prompt(`Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    const deployments = {
        tokenManagerDeployer: {
            name: 'Token Manager Deployer',
            async deploy() {
                return await deployContract(
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
        interchainToken: {
            name: 'Interchain Token',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('InterchainToken', artifactPath),
                    [interchainTokenService],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        interchainTokenDeployer: {
            name: 'Interchain Token Deployer',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('InterchainTokenDeployer', artifactPath),
                    [contractConfig.interchainToken],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        tokenManager: {
            name: 'Token Manager',
            async deploy() {
                return await deployContract(
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
            async deploy() {
                return await deployContract(
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
            async deploy() {
                return await deployContract(
                    proxyDeployMethod,
                    wallet,
                    InterchainTokenService,
                    [
                        contractConfig.tokenManagerDeployer,
                        contractConfig.interchainTokenDeployer,
                        contracts.AxelarGateway.address,
                        contracts.AxelarGasService.address,
                        interchainTokenFactory,
                        chain.id,
                        contractConfig.tokenManager,
                        contractConfig.tokenHandler,
                    ],
                    getDeployOptions(proxyDeployMethod, implementationSalt, chain),
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        address: {
            name: 'Interchain Token Service Proxy',
            async deploy() {
                const operatorAddress = options.operatorAddress || wallet.address;

                const deploymentParams = defaultAbiCoder.encode(
                    ['address', 'string', 'string[]', 'string[]'],
                    [operatorAddress, chain.id, trustedChains, trustedAddresses],
                );
                contractConfig.predeployCodehash = predeployCodehash;

                return await deployContract(
                    proxyDeployMethod,
                    wallet,
                    getContractJSON('InterchainProxy', artifactPath),
                    [contractConfig.implementation, wallet.address, deploymentParams],
                    getDeployOptions(proxyDeployMethod, proxySalt, chain),
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        interchainTokenFactoryImplementation: {
            name: 'Interchain Token Factory Implementation',
            async deploy() {
                return await deployContract(
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
            async deploy() {
                return await deployContract(
                    proxyDeployMethod,
                    wallet,
                    getContractJSON('InterchainProxy', artifactPath),
                    [contractConfig.interchainTokenFactoryImplementation, wallet.address, '0x'],
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

        if ((key === 'address' || key === 'interchainTokenFactory') && options.reuseProxy) {
            printInfo(`Reusing ${deployment.name} deployment at ${contractConfig[key]}`);
            continue;
        }

        printInfo(`Deploying ${deployment.name}`);

        const contract = await deployment.deploy();
        contractConfig[key] = contract.address;
        printInfo(`Deployed ${deployment.name} at ${contract.address}`);

        saveConfig(config, options.env);

        if (chain.chainId !== 31337) {
            await sleep(2000);
        }

        if (!(await isContract(contract.address, provider))) {
            throw new Error(`Contract ${deployment.name} at ${contract.address} was not deployed on ${chain.name}`);
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

async function upgrade(config, chain, options) {
    const { artifactPath, privateKey } = options;

    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet(privateKey, provider);
    const contractName = 'InterchainTokenService';

    await printWalletInfo(wallet, options);

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contracts[contractName] = contractConfig;

    printInfo(`Upgrading Interchain Token Service.`);

    const InterchainTokenService = getContractJSON('InterchainTokenService', artifactPath);
    const gasOptions = await getGasOptions(chain, options, contractName);
    const contract = new Contract(contractConfig.address, InterchainTokenService.abi, wallet);
    const codehash = await getBytecodeHash(contractConfig.implementation, chain.id, provider);

    printInfo(`ITS Proxy`, contract.address);

    const currImplementation = await contract.implementation();
    printInfo(`Current ITS implementation`, currImplementation);
    printInfo(`New ITS implementation`, contractConfig.implementation);

    if (prompt(`Proceed with ITS upgrade on ${chain.name}?`, options.yes)) {
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

    const InterchainTokenFactory = getContractJSON('InterchainTokenFactory', artifactPath);
    const itsFactory = new Contract(contractConfig.interchainTokenFactory, InterchainTokenFactory.abi, wallet);
    const factoryCodehash = await getBytecodeHash(contractConfig.interchainTokenFactoryImplementation, chain.id, provider);

    printInfo(`ITS Factory Proxy`, itsFactory.address);

    const factoryImplementation = await itsFactory.implementation();
    printInfo(`Current ITS Factory implementation`, factoryImplementation);
    printInfo(`New ITS Factory implementation`, contractConfig.interchainTokenFactoryImplementation);

    if (
        prompt(
            `Proceed with ITS Factory upgrade to implementation ${contractConfig.interchainTokenFactoryImplementation} on ${chain.name}?`,
            options.yes,
        )
    ) {
        return;
    }

    const factoryReceipt = await itsFactory
        .upgrade(contractConfig.interchainTokenFactoryImplementation, factoryCodehash, '0x', gasOptions)
        .then((tx) => tx.wait(chain.confirmations));

    if (!wasEventEmitted(factoryReceipt, itsFactory, 'Upgraded')) {
        printError('Upgrade failed');
        return;
    }

    printInfo(`Upgraded Interchain Token Factory`);
}

async function processCommand(config, chain, options) {
    if (options.upgrade) {
        await upgrade(config, chain, options);
    } else {
        await deploy(config, chain, options);
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-its').description('Deploy interchain token service');

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

    addExtendedOptions(program, { skipExisting: true, upgrade: true });

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

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS: deploy };
}
