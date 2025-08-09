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
    isHyperliquidChain,
    parseTrustedChains,
    detectITSVersion,
} = require('./utils');
const { itsHubContractAddress } = require('../common/utils');
const { addEvmOptions } = require('./cli-utils');
const { Command, Option } = require('commander');
const { updateBlockSize } = require('./hyperliquid');

/**
 * Function that handles the ITS deployment with chain-specific token support.
 * @param {*} wallet
 * @param {*} chain
 * @param {*} deployOptions
 * @param {*} operatorAddress
 * @param {*} skipExisting
 * @param {*} verifyOptions
 */

async function deployAll(axelar, wallet, chain, chains, options) {
    const { env, artifactPath, deployMethod, proxyDeployMethod, skipExisting, verify, yes, predictOnly } = options;
    const verifyOptions = verify ? { env, chain: chain.axelarId, only: verify === 'only' } : null;

    const provider = getDefaultProvider(chain.rpc);

    const contractName = 'InterchainTokenService';
    const itsFactoryContractName = 'InterchainTokenFactory';
    const contracts = chain.contracts;

    // Deploy only the appropriate token implementation based on chain type
    const interchainTokenContractName = isHyperliquidChain(chain) ? 'HyperliquidInterchainToken' : 'InterchainToken';
    const InterchainTokenService = getContractJSON(
        isHyperliquidChain(chain) ? 'HyperliquidInterchainTokenService' : 'InterchainTokenService',
        artifactPath,
    );

    const contractConfig = contracts[contractName] || {};
    const itsFactoryContractConfig = contracts[itsFactoryContractName] || {};

    const itsVersion = detectITSVersion();

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

    const proxyJSON = getContractJSON('InterchainProxy', artifactPath);
    const predeployCodehash = await getBytecodeHash(proxyJSON, chain.axelarId);
    const gasOptions = await getGasOptions(chain, options, contractName);
    const deployOptions = getDeployOptions(deployMethod, salt, chain);

    const interchainTokenService = (contractConfig['address'] = options.reuseProxy
        ? contractConfig.address
        : await getDeployedAddress(wallet.address, proxyDeployMethod, {
              salt: proxySalt,
              deployerContract: getDeployOptions(proxyDeployMethod, proxySalt, chain).deployerContract,
              contractJson: proxyJSON,
              constructorArgs: [],
              provider: wallet.provider,
          }));

    const interchainTokenFactory = (itsFactoryContractConfig['address'] = options.reuseProxy
        ? itsFactoryContractConfig.address
        : await getDeployedAddress(wallet.address, proxyDeployMethod, {
              salt: factorySalt,
              deployerContract: getDeployOptions(proxyDeployMethod, factorySalt, chain).deployerContract,
              contractJSON: proxyJSON,
              constructorArgs: [],
              provider: wallet.provider,
          }));

    if (options.reuseProxy) {
        if (!isValidAddress(interchainTokenService) || !isValidAddress(interchainTokenFactory)) {
            printError('No ITS contract found for chain', chain.name);
            return;
        }

        printInfo('Reusing existing Interchain Token Service proxy', interchainTokenService);
        printInfo('Reusing existing Interchain Token Factory proxy', interchainTokenFactory);
    } else {
        printInfo('Interchain Token Service will be deployed to', interchainTokenService);
        printInfo('Interchain Token Factory will be deployed to', interchainTokenFactory);
    }

    contracts[contractName] = contractConfig;
    contracts[itsFactoryContractName] = itsFactoryContractConfig;

    const trustedChains = parseTrustedChains(chains, ['all']);
    const itsHubAddress = itsHubContractAddress(axelar);

    // Trusted addresses are only used when deploying a new proxy
    if (!options.reuseProxy) {
        printInfo('Trusted chains', trustedChains);
    }

    const existingAddress = chains.ethereum?.contracts?.[contractName]?.address;

    if (existingAddress !== undefined && interchainTokenService !== existingAddress) {
        printWarn(
            `Predicted address ${interchainTokenService} does not match existing deployment ${existingAddress} on chain ${chains.ethereum.name}`,
        );

        const existingCodeHash = chains.ethereum.contracts[contractName].predeployCodehash;

        if (predeployCodehash !== existingCodeHash) {
            printWarn(
                `Pre-deploy bytecode hash ${predeployCodehash} does not match existing deployment's predeployCodehash ${existingCodeHash} on chain ${chains.ethereum.name}`,
            );
        }

        printWarn('For official deployment, recheck the deployer, salt, args, or contract bytecode');
    }

    if (predictOnly || prompt(`Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    contractConfig.version = itsVersion;
    itsFactoryContractConfig.version = itsVersion;

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
        interchainToken: {
            name: 'Interchain Token',
            contractName: 'InterchainToken',
            async deploy() {
                return deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON(interchainTokenContractName, artifactPath),
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
            contractName: 'InterchainTokenDeployer',
            async deploy() {
                return deployContract(
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
            useHyperliquidBigBlocks: isHyperliquidChain(chain),
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

                const deploymentParams = defaultAbiCoder.encode(
                    ['address', 'string', 'string[]'],
                    [operatorAddress, chain.axelarId, trustedChains],
                );
                contractConfig.predeployCodehash = predeployCodehash;

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
            useHyperliquidBigBlocks: isHyperliquidChain(chain),
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

        if (deployment.useHyperliquidBigBlocks) {
            await updateBlockSize(wallet, chain, true);
        }

        printInfo(`Deploying ${deployment.name}`);

        let contract;
        try {
            contract = await deployment.deploy();
        } finally {
            if (deployment.useHyperliquidBigBlocks) {
                await updateBlockSize(wallet, chain, false);
            }
        }

        if (key === 'interchainTokenFactoryImplementation') {
            itsFactoryContractConfig.implementation = contract.address;
        } else if (key === 'interchainTokenFactory') {
            itsFactoryContractConfig.address = contract.address;
        } else {
            contractConfig[key] = contract.address;
        }

        printInfo(`Deployed ${deployment.name} at ${contract.address}`);

        if (chain.chainId !== 31337) {
            await sleep(5000);
        }

        if (!(await isContract(contract.address, provider))) {
            throw new Error(`Contract ${deployment.name} at ${contract.address} was not deployed on ${chain.name}`);
        }
    }
}

async function deploy(axelar, chain, chains, options) {
    const { privateKey, salt } = options;

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);

    await printWalletInfo(wallet, options);

    const operatorAddress = options.operatorAddress || wallet.address;

    if (!isAddress(operatorAddress)) {
        throw new Error(`Invalid operator address: ${operatorAddress}`);
    }

    await deployAll(axelar, wallet, chain, chains, options);
}

async function upgrade(_axelar, chain, _chains, options) {
    const { artifactPath, privateKey, predictOnly } = options;

    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet(privateKey, provider);
    const contractName = 'InterchainTokenService';
    const itsFactoryContractName = 'InterchainTokenFactory';

    await printWalletInfo(wallet, options);

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];
    const itsFactoryContractConfig = contracts[itsFactoryContractName];

    if (!contractConfig || !itsFactoryContractConfig) {
        printError('No ITS contract found for chain', chain.name);
        return;
    }

    const itsVersion = detectITSVersion();

    printInfo(`Upgrading Interchain Token Service on ${chain.name} to version ${itsVersion}.`);

    const InterchainTokenService = getContractJSON(
        isHyperliquidChain(chain) ? 'HyperliquidInterchainTokenService' : 'InterchainTokenService',
        artifactPath,
    );
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

    contractConfig.version = itsVersion;
    itsFactoryContractConfig.version = itsVersion;

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

async function processCommand(axelar, chain, chains, options) {
    if (options.upgrade) {
        await upgrade(axelar, chain, chains, options);
    } else {
        await deploy(axelar, chain, chains, options);
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-its')
        .description('Deploy interchain token service and interchain token factory with chain-specific token support');

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

    addEvmOptions(program, { artifactPath: true, skipExisting: true, upgrade: true, predictOnly: true });

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
