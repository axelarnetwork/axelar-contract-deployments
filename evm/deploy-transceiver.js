'use strict';

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, printWarn, saveConfig, mainProcessor, getContractJSON } = require('./utils');
const { addEvmOptions } = require('./cli-utils');

// Import the deployEvmContract function from deploy-contract.js
const { deployEvmContract } = require('./deploy-contract');

/**
 * Deploys TransceiverStructs library using the generic deployEvmContract function.
 *
 * @param {Object} config - The global configuration object
 * @param {Object} chain - The chain configuration object
 * @param {Object} options - Deployment options
 * @returns {Promise<Object|null>} The deployed contract or null if cancelled
 */
async function deployTransceiverStructs(config, chain, options) {
    // Create a modified options object for TransceiverStructs deployment
    const structsOptions = {
        ...options,
        contractName: 'TransceiverStructs',
        salt: options.transceiverStructsSalt || 'TransceiverStructs',
    };

    // Use the generic deployment function
    const contract = await deployEvmContract(config, chain, structsOptions);

    return contract;
}

/**
 * Links the TransceiverStructs library to the AxelarTransceiver bytecode.
 *
 * @param {Object} transceiverJson - The contract JSON object
 * @param {string} libraryAddress - The library address to link
 * @returns {Object} The modified contract JSON with linked library
 */
async function linkLibraryToTransceiver(transceiverJson, libraryAddress) {
    // Replace library placeholder in bytecode
    const libraryPlaceholder = '__$' + 'TransceiverStructs'.padEnd(38, '$') + '__';
    const libraryAddressPadded = libraryAddress.slice(2).padStart(40, '0');
    transceiverJson.bytecode = transceiverJson.bytecode.replace(libraryPlaceholder, libraryAddressPadded);
    return transceiverJson;
}

/**
 * Deploys AxelarTransceiver proxy contract using the generic deployEvmContract function.
 *
 * @param {Object} config - The global configuration object
 * @param {Object} chain - The chain configuration object
 * @param {Object} options - Deployment options
 * @param {string} implementationAddress - The implementation contract address
 * @returns {Promise<Object|null>} The deployed proxy contract or null if cancelled
 */
async function deployAxelarTransceiverProxy(config, chain, options, implementationAddress) {
    const { reuseProxy } = options;

    if (!chain.contracts.AxelarTransceiver) {
        chain.contracts.AxelarTransceiver = {};
    }

    const transceiverConfig = chain.contracts.AxelarTransceiver;

    // Check if we should reuse existing proxy
    if (reuseProxy && transceiverConfig.proxyAddress) {
        printWarn(`Reusing existing proxy at ${transceiverConfig.proxyAddress}`);
        // Create contract instance for the existing proxy
        const provider = getDefaultProvider(chain.rpc);
        const wallet = new Wallet(options.privateKey, provider);
        const contractJson = getContractJSON('AxelarTransceiver', options.artifactPath);
        return new ethers.Contract(transceiverConfig.proxyAddress, contractJson.abi, wallet);
    }

    if (transceiverConfig.proxyAddress && options.skipExisting) {
        printWarn(`Skipping AxelarTransceiver proxy deployment on ${chain.name} because it is already deployed.`);
        // Create contract instance for the existing proxy
        const provider = getDefaultProvider(chain.rpc);
        const wallet = new Wallet(options.privateKey, provider);
        const contractJson = getContractJSON('AxelarTransceiver', options.artifactPath);
        return new ethers.Contract(transceiverConfig.proxyAddress, contractJson.abi, wallet);
    }

    // Create a modified options object for ERC1967Proxy deployment
    const proxyOptions = {
        ...options,
        contractName: 'ERC1967Proxy',
        deployMethod: options.proxyDeployMethod || 'create3',
        salt: options.proxySalt || 'AxelarTransceiverProxy v1.0.0',
        args: JSON.stringify([implementationAddress, '0x']), // implementation address and empty init data
    };

    // Use the generic deployment function
    const proxyContract = await deployEvmContract(config, chain, proxyOptions);

    // Create AxelarTransceiver contract instance pointing to the proxy
    if (proxyContract) {
        const contractJson = getContractJSON('AxelarTransceiver', options.artifactPath);
        return new ethers.Contract(proxyContract.address, contractJson.abi, proxyContract.signer);
    }

    return null;
}

/**
 * Deploys AxelarTransceiver implementation contract using the generic deployEvmContract function.
 *
 * @param {Object} config - The global configuration object
 * @param {Object} chain - The chain configuration object
 * @param {Object} options - Deployment options
 * @param {string} libraryAddress - The TransceiverStructs library address
 * @returns {Promise<Object|null>} The deployed contract or null if cancelled
 */
async function deployAxelarTransceiver(config, chain, options, libraryAddress) {
    // Create a modified options object for AxelarTransceiver deployment
    const transceiverOptions = {
        ...options,
        contractName: 'AxelarTransceiver',
        salt: options.transceiverSalt || 'AxelarTransceiver',
        args: JSON.stringify({
            gateway: chain.contracts.AxelarGateway?.address,
            gasService: chain.contracts.AxelarGasService?.address,
            nttManager: options.nttManager,
        }),
    };

    // Use the generic deployment function
    const contract = await deployEvmContract(config, chain, transceiverOptions);

    return contract;
}

/**
 * Initializes the AxelarTransceiver contract if not already initialized.
 *
 * @param {Object} transceiverContract - The transceiver contract instance
 * @returns {Promise<void>}
 */
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

/**
 * Transfers pauser capability to the specified address.
 *
 * @param {Object} transceiverContract - The transceiver contract instance
 * @param {string} pauserAddress - The address to transfer pauser capability to
 * @returns {Promise<void>}
 */
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

/**
 * Processes the transceiver deployment command for a specific chain.
 * Orchestrates the deployment of TransceiverStructs, AxelarTransceiver, and proxy.
 *
 * @param {Object} config - The global configuration object
 * @param {Object} chain - The chain configuration object
 * @param {Object} options - Deployment options
 * @returns {Promise<void>}
 */
async function processCommand(config, chain, options) {
    // Deploy TransceiverStructs library first
    const structsContract = await deployTransceiverStructs(config, chain, options);
    if (!structsContract) {
        return; // User cancelled or predictOnly mode
    }

    // Deploy AxelarTransceiver implementation
    const implementationContract = await deployAxelarTransceiver(config, chain, options, structsContract.address);
    if (!implementationContract) {
        return; // User cancelled or predictOnly mode
    }

    // Deploy proxy if not reusing existing one
    const finalContract = await deployAxelarTransceiverProxy(config, chain, options, implementationContract.address);
    if (!finalContract) {
        return; // User cancelled or predictOnly mode
    }

    // Initialize the contract if needed
    await initializeTransceiver(finalContract);

    // Transfer pauser capability if provided
    await transferPauserCapability(finalContract, options.pauserAddress);

    saveConfig(config, options.env);
}

/**
 * Main entry point for the deploy-transceiver script.
 *
 * @param {Object} options - Command line options and configuration
 * @returns {Promise<void>}
 */
async function main(options) {
    await mainProcessor(options, processCommand);
}

// CLI setup and execution
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

    program.addOption(new Option('--nttManager <nttManager>', 'NTT Manager address').makeOptionMandatory(true).env('NTT_MANAGER'));

    program.addOption(new Option('--pauserAddress <pauserAddress>', 'Address to transfer pauser capability to').env('PAUSER_ADDRESS'));

    program.addOption(
        new Option('--transceiverSalt <transceiverSalt>', 'deployment salt to use for AxelarTransceiver deployment').env(
            'TRANSCEIVER_SALT',
        ),
    );

    program.addOption(
        new Option('--transceiverStructsSalt <transceiverStructsSalt>', 'deployment salt to use for TransceiverStructs deployment').env(
            'TRANSCEIVER_STRUCTS_SALT',
        ),
    );

    program.addOption(
        new Option('--proxySalt <proxySalt>', 'deployment salt to use for AxelarTransceiver proxy deployment')
            .default('AxelarTransceiverProxy v1.0.0')
            .env('PROXY_SALT'),
    );

    program.addOption(new Option('--reuseProxy', 'reuse existing proxy (useful for upgrade deployments'));

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
} else {
    module.exports = {
        deployTransceiver: main,
        linkLibraryToTransceiver,
    };
}
