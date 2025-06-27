'use strict';

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    printWarn,
    printError,
    getGasOptions,
    getBytecodeHash,
    printWalletInfo,
    getDeployedAddress,
    deployContract,
    saveConfig,
    prompt,
    mainProcessor,
    isContract,
    getContractJSON,
    getDeployOptions,
} = require('./utils');
const { addEvmOptions } = require('./cli-utils');

async function getTransceiverConstructorArgs(config, options) {
    const gateway = config.AxelarGateway?.address;
    const gasService = config.AxelarGasService?.address;
    const nttManager = options.nttManager;

    if (!isAddress(gateway)) {
        throw new Error(`Missing AxelarGateway address in the chain info.`);
    }

    if (!isAddress(gasService)) {
        throw new Error(`Missing AxelarGasService address in the chain info.`);
    }

    if (!isAddress(nttManager)) {
        throw new Error(`Missing NTT Manager address. Please provide --nttManager parameter.`);
    }

    return [gateway, gasService, nttManager];
}

async function getTransceiverStructsConstructorArgs() {
    // TransceiverStructs library has no constructor arguments
    return [];
}

async function checkTransceiverContract(contract, contractConfig) {
    const gateway = await contract.gateway();
    const gasService = await contract.gasService();
    const nttManager = await contract.nttManager();

    if (gateway !== contractConfig.gateway) {
        printError(`Expected gateway ${contractConfig.gateway} but got ${gateway}.`);
    }

    if (gasService !== contractConfig.gasService) {
        printError(`Expected gasService ${contractConfig.gasService} but got ${gasService}.`);
    }

    if (nttManager !== contractConfig.nttManager) {
        printError(`Expected nttManager ${contractConfig.nttManager} but got ${nttManager}.`);
    }

    printInfo('Transceiver contract verification passed');
}

async function deployTransceiverStructs(config, chain, options) {
    const { env, artifactPath, deployMethod, privateKey, verify, yes, predictOnly } = options;
    const verifyOptions = verify ? { env, chain: chain.name, only: verify === 'only' } : null;

    if (!chain.contracts) {
        chain.contracts = {};
    }

    if (!chain.contracts.TransceiverStructs) {
        chain.contracts.TransceiverStructs = {};
    }

    const transceiverStructsConfig = chain.contracts.TransceiverStructs;

    if (transceiverStructsConfig.address && options.skipExisting) {
        printWarn(`Skipping TransceiverStructs deployment on ${chain.name} because it is already deployed.`);
        return transceiverStructsConfig.address;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Deploying TransceiverStructs library...');

    const transceiverStructsJson = getContractJSON('TransceiverStructs', artifactPath);
    const transceiverStructsArgs = await getTransceiverStructsConstructorArgs();
    const gasOptions = await getGasOptions(chain, options, 'TransceiverStructs');

    const { deployerContract, salt } = getDeployOptions(deployMethod, options.transceiverStructsSalt || 'TransceiverStructs', chain);

    const predictedStructsAddress = await getDeployedAddress(wallet.address, deployMethod, {
        salt,
        deployerContract,
        contractJson: transceiverStructsJson,
        constructorArgs: transceiverStructsArgs,
        provider: wallet.provider,
    });

    if (await isContract(predictedStructsAddress, provider)) {
        printWarn(`TransceiverStructs is already deployed on ${chain.name} at ${predictedStructsAddress}`);
        transceiverStructsConfig.address = predictedStructsAddress;
        return predictedStructsAddress;
    }

    if (predictOnly || prompt(`Proceed with TransceiverStructs deployment on ${chain.name}?`, yes)) {
        return null;
    }

    const transceiverStructsContract = await deployContract(
        deployMethod,
        wallet,
        transceiverStructsJson,
        transceiverStructsArgs,
        { salt, deployerContract },
        gasOptions,
        verifyOptions,
        chain,
    );

    const codehash = await getBytecodeHash(transceiverStructsContract, chain.axelarId);
    printInfo('Deployed TransceiverStructs bytecode hash', codehash);

    transceiverStructsConfig.address = transceiverStructsContract.address;
    transceiverStructsConfig.deployer = wallet.address;
    transceiverStructsConfig.deploymentMethod = deployMethod;
    transceiverStructsConfig.codehash = codehash;

    if (deployMethod !== 'create') {
        transceiverStructsConfig.salt = salt;
    }

    printInfo(`${chain.name} | TransceiverStructs`, transceiverStructsConfig.address);
    return transceiverStructsContract.address;
}

async function linkLibraryToTransceiver(transceiverJson, libraryAddress) {
    // Replace library placeholder in bytecode
    const libraryPlaceholder = '__$' + 'TransceiverStructs'.padEnd(38, '$') + '__';
    const libraryAddressPadded = libraryAddress.slice(2).padStart(40, '0');
    transceiverJson.bytecode = transceiverJson.bytecode.replace(libraryPlaceholder, libraryAddressPadded);
    return transceiverJson;
}

async function deployAxelarTransceiverProxy(config, chain, options, implementationAddress) {
    const { artifactPath, proxyDeployMethod, privateKey, verify, yes, predictOnly, reuseProxy } = options;
    const verifyOptions = verify ? { env: options.env, chain: chain.name, only: verify === 'only' } : null;

    if (!chain.contracts.AxelarTransceiver) {
        chain.contracts.AxelarTransceiver = {};
    }

    const transceiverConfig = chain.contracts.AxelarTransceiver;

    // Check if we should reuse existing proxy
    if (reuseProxy && transceiverConfig.proxyAddress) {
        printWarn(`Reusing existing proxy at ${transceiverConfig.proxyAddress}`);
        return transceiverConfig.proxyAddress;
    }

    if (transceiverConfig.proxyAddress && options.skipExisting) {
        printWarn(`Skipping AxelarTransceiver proxy deployment on ${chain.name} because it is already deployed.`);
        return transceiverConfig.proxyAddress;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Deploying AxelarTransceiver proxy...');

    const proxyJson = getContractJSON('ERC1967Proxy', artifactPath);
    const proxyArgs = [implementationAddress, '0x']; // implementation address and empty init data
    const gasOptions = await getGasOptions(chain, options, 'ERC1967Proxy');

    const { deployerContract, salt } = getDeployOptions(proxyDeployMethod, options.proxySalt || 'AxelarTransceiverProxy', chain);

    const predictedProxyAddress = await getDeployedAddress(wallet.address, proxyDeployMethod, {
        salt,
        deployerContract,
        contractJson: proxyJson,
        constructorArgs: proxyArgs,
        provider: wallet.provider,
    });

    if (await isContract(predictedProxyAddress, provider)) {
        printWarn(`AxelarTransceiver proxy is already deployed on ${chain.name} at ${predictedProxyAddress}`);
        transceiverConfig.proxyAddress = predictedProxyAddress;
        return predictedProxyAddress;
    }

    if (predictOnly || prompt(`Proceed with AxelarTransceiver proxy deployment on ${chain.name}?`, yes)) {
        return null;
    }

    const proxyContract = await deployContract(
        proxyDeployMethod,
        wallet,
        proxyJson,
        proxyArgs,
        { salt, deployerContract },
        gasOptions,
        verifyOptions,
        chain,
    );

    const codehash = await getBytecodeHash(proxyContract, chain.axelarId);
    printInfo('Deployed AxelarTransceiver proxy bytecode hash', codehash);

    transceiverConfig.proxyAddress = proxyContract.address;
    transceiverConfig.proxyDeployer = wallet.address;
    transceiverConfig.proxyDeploymentMethod = proxyDeployMethod;
    transceiverConfig.proxyCodehash = codehash;

    if (proxyDeployMethod !== 'create') {
        transceiverConfig.proxySalt = salt;
    }

    printInfo(`${chain.name} | AxelarTransceiver Proxy`, transceiverConfig.proxyAddress);

    return proxyContract;
}

async function deployAxelarTransceiver(config, chain, options, libraryAddress) {
    const { artifactPath, deployMethod, privateKey, verify, yes, predictOnly, nttManager, pauserAddress } = options;
    const verifyOptions = verify ? { env: options.env, chain: chain.name, only: verify === 'only' } : null;

    if (!chain.contracts.AxelarTransceiver) {
        chain.contracts.AxelarTransceiver = {};
    }

    const transceiverConfig = chain.contracts.AxelarTransceiver;

    if (transceiverConfig.implementationAddress && options.skipExisting) {
        printWarn(`Skipping AxelarTransceiver implementation deployment on ${chain.name} because it is already deployed.`);
        return transceiverConfig.implementationAddress;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Deploying AxelarTransceiver implementation...');

    const transceiverJson = getContractJSON('AxelarTransceiver', artifactPath);
    
    // Link the library
    if (!libraryAddress) {
        throw new Error('TransceiverStructs library address not found. Deploy it first.');
    }

    linkLibraryToTransceiver(transceiverJson, libraryAddress);

    const transceiverArgs = await getTransceiverConstructorArgs(chain.contracts, wallet, options);
    const gasOptions = await getGasOptions(chain, options, 'AxelarTransceiver');

    const { deployerContract, salt } = getDeployOptions(deployMethod, options.transceiverSalt || 'AxelarTransceiver', chain);

    const predictedTransceiverAddress = await getDeployedAddress(wallet.address, deployMethod, {
        salt,
        deployerContract,
        contractJson: transceiverJson,
        constructorArgs: transceiverArgs,
        provider: wallet.provider,
    });

    if (await isContract(predictedTransceiverAddress, provider)) {
        printWarn(`AxelarTransceiver implementation is already deployed on ${chain.name} at ${predictedTransceiverAddress}`);
        transceiverConfig.implementationAddress = predictedTransceiverAddress;
        return predictedTransceiverAddress;
    }

    if (predictOnly || prompt(`Proceed with AxelarTransceiver implementation deployment on ${chain.name}?`, yes)) {
        return null;
    }

    const transceiverContract = await deployContract(
        deployMethod,
        wallet,
        transceiverJson,
        transceiverArgs,
        { salt, deployerContract },
        gasOptions,
        verifyOptions,
        chain,
    );

    const codehash = await getBytecodeHash(transceiverContract, chain.axelarId);
    printInfo('Deployed AxelarTransceiver implementation bytecode hash', codehash);

    transceiverConfig.implementationAddress = transceiverContract.address;
    transceiverConfig.implementationDeployer = wallet.address;
    transceiverConfig.implementationDeploymentMethod = deployMethod;
    transceiverConfig.implementationCodehash = codehash;
    transceiverConfig.gateway = transceiverArgs[0];
    transceiverConfig.gasService = transceiverArgs[1];
    transceiverConfig.nttManager = transceiverArgs[2];

    if (deployMethod !== 'create') {
        transceiverConfig.implementationSalt = salt;
    }

    printInfo(`${chain.name} | AxelarTransceiver Implementation`, transceiverConfig.implementationAddress);

    // Verify the contract configuration
    await checkTransceiverContract(transceiverContract, transceiverConfig, options);

    return transceiverContract;
}

async function initializeTransceiver(transceiverContract) {
    try {
        const isInitialized = await transceiverContract.isInitialized();
        if (!isInitialized) {
            printInfo('Initializing AxelarTransceiver...');
            const initTx = await transceiverContract.initialize();
            await initTx.wait();
            printInfo('AxelarTransceiver initialized successfully');
        }
    } catch (error) {
        printWarn('Could not check or initialize transceiver:', error.message);
    }
}

async function transferPauserCapability(transceiverContract, pauserAddress) {
    if (pauserAddress && isAddress(pauserAddress)) {
        try {
            printInfo(`Transferring pauser capability to ${pauserAddress}...`);
            const transferTx = await transceiverContract.transferPauserCapability(pauserAddress);
            await transferTx.wait();
            printInfo('Pauser capability transferred successfully');
        } catch (error) {
            printWarn('Could not transfer pauser capability:', error.message);
        }
    }
}

async function processCommand(config, chain, options) {
    // Deploy TransceiverStructs library first
    const libraryAddress = await deployTransceiverStructs(config, chain, options);
    if (!libraryAddress) {
        return; // User cancelled or predictOnly mode
    }

    // Deploy AxelarTransceiver implementation
    const implementationContract = await deployAxelarTransceiver(config, chain, options, libraryAddress);
    if (!implementationContract) {
        return; // User cancelled or predictOnly mode
    }

    // Deploy proxy if not reusing existing one
    const proxyContract = await deployAxelarTransceiverProxy(config, chain, options, implementationContract.address);
    if (!proxyContract) {
        return; // User cancelled or predictOnly mode
    }

    // Get the final contract (proxy if deployed, implementation if no proxy)
    const finalContract = proxyContract || implementationContract;

    // Initialize the contract if needed
    await initializeTransceiver(finalContract);

    // Transfer pauser capability if provided
    await transferPauserCapability(finalContract, options.pauserAddress);

    saveConfig(config, options.env);
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-transceiver').description('Deploy AxelarTransceiver and TransceiverStructs library');

    addEvmOptions(program, {
        artifactPath: true,
        skipExisting: true,
        predictOnly: true,
    });

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

    program.addOption(
        new Option('--nttManager <nttManager>', 'NTT Manager address').makeOptionMandatory(true).env('NTT_MANAGER'),
    );

    program.addOption(
        new Option('--pauserAddress <pauserAddress>', 'Address to transfer pauser capability to').env('PAUSER_ADDRESS'),
    );

    program.addOption(
        new Option('--transceiverSalt <transceiverSalt>', 'deployment salt to use for AxelarTransceiver deployment').env('TRANSCEIVER_SALT'),
    );

    program.addOption(
        new Option('--transceiverStructsSalt <transceiverStructsSalt>', 'deployment salt to use for TransceiverStructs deployment').env('TRANSCEIVER_STRUCTS_SALT'),
    );

    program.addOption(
        new Option(
            '--proxySalt <proxySalt>',
            'deployment salt to use for AxelarTransceiver proxy deployment',
        )
            .default('AxelarTransceiverProxy v1.0.0')
            .env('PROXY_SALT'),
    );

    program.addOption(new Option('--reuseProxy', 'reuse existing proxy (useful for upgrade deployments'));

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployTransceiver: main };
}
