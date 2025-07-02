'use strict';

/**
 * @fileoverview EVM Transceiver Deployment Script
 *
 * This script provides functionality to deploy AxelarTransceiver contracts and their dependencies
 * on EVM-compatible chains. It orchestrates the deployment of TransceiverStructs library,
 * AxelarTransceiver implementation.
 *
 * Deployment sequence:
 * 1. TransceiverStructs library (required by AxelarTransceiver)
 * 2. AxelarTransceiver implementation contract
 * 4. Contract initialization and pauser capability transfer
 *
 * @requires hardhat
 * @requires ethers
 * @requires commander
 * @requires ./utils
 * @requires ./cli-utils
 * @requires ./deploy-upgradable
 */

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, printWarn, saveConfig, mainProcessor, getContractJSON } = require('./utils');
const { addEvmOptions } = require('./cli-utils');

// Import the deployEvmUpgradableContract function from deploy-upgradable.js
const { deployEvmUpgradableContract } = require('./deploy-upgradable');

/**
 * Deploys TransceiverStructs library using the generic deployEvmUpgradableContract function.
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
    const contract = await deployEvmUpgradableContract(config, chain, structsOptions);

    return contract;
}

/**
 * Deploys AxelarTransceiver implementation contract using the generic deployEvmUpgradableContract function.
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
            libraryAddress: libraryAddress,
        }),
    };

    // Use the generic deployment function
    const contract = await deployEvmUpgradableContract(config, chain, transceiverOptions);

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

            // TODO tkulik: How to handle the pauser capability?
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
async function deployTransceiverContracts(config, chain, options) {
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

    // Initialize the contract if needed
    await initializeTransceiver(implementationContract);

    // Transfer pauser capability if provided
    await transferPauserCapability(implementationContract, options.pauserAddress);

    saveConfig(config, options.env);
}

/**
 * Main entry point for the deploy-transceiver script.
 *
 * @param {Object} options - Command line options and configuration
 * @returns {Promise<void>}
 */
async function main(options) {
    await mainProcessor(options, deployTransceiverContracts);
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

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
}
