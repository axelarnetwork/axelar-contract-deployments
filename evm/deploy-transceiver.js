'use strict';

// TODO tkulik:
// * Update README.md to reflect the new transceiver deployment process
// * Remove this file once we confirm that we don't want to have a single script to deploy
//   everything related to transceivers.

/**
 * @fileoverview EVM Transceiver Deployment Script
 *
 * This script provides functionality to deploy AxelarTransceiver contracts and their dependencies
 * on EVM-compatible chains. It orchestrates the deployment of AxelarTransceiver implementation
 * and ERC1967Proxy. The TransceiverStructs library is statically linked at compile time.
 *
 * Deployment sequence:
 * 1. AxelarTransceiver implementation contract (with statically linked library)
 * 2. ERC1967Proxy contract
 * 3. Contract initialization and pauser capability transfer
 */

const { ethers } = require('hardhat');
const {
    utils: { isAddress },
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, printWarn, saveConfig, mainProcessor, getContractJSON } = require('./utils');
const { addEvmOptions } = require('./cli-utils');
const { processCommand: deployEvmContract } = require('./deploy-contract');

/**
 * Deploys the AxelarTransceiver implementation contract.
 */
async function deployAxelarTransceiver(config, chain, options) {
    const transceiverOptions = {
        ...options,
        contractName: 'AxelarTransceiver',
        salt: options.transceiverSalt || 'AxelarTransceiver',
        args: JSON.stringify({
            gateway: chain.contracts.AxelarGateway?.address,
            gasService: chain.contracts.AxelarGasService?.address,
            gmpManager: options.gmpManager,
        }),
    };

    const contract = await deployEvmContract(config, chain, transceiverOptions);

    return contract;
}

/**
 * Deploys the ERC1967Proxy contract for AxelarTransceiver.
 */
async function deployTransceiverProxy(config, chain, options, implementationAddress) {
    const proxyOptions = {
        ...options,
        contractName: 'ERC1967Proxy',
        salt: options.proxySalt || 'AxelarTransceiverProxy',
        args: JSON.stringify([implementationAddress, '0x']), // implementation address and empty init data
        forContract: 'AxelarTransceiver', // Specify that this proxy is for AxelarTransceiver
    };

    const contract = await deployEvmContract(config, chain, proxyOptions);

    return contract;
}

/**
 * Creates an AxelarTransceiver interface instance using the proxy address.
 */
function createTransceiverInterface(proxyAddress, artifactPath) {
    const transceiverJson = getContractJSON('AxelarTransceiver', artifactPath);
    const { ethers } = require('hardhat');
    const { Contract } = ethers;

    // Create a contract instance with AxelarTransceiver ABI but proxy address
    return new Contract(proxyAddress, transceiverJson.abi, null);
}

/**
 * Initializes the AxelarTransceiver contract if not already initialized.
 */
async function initializeTransceiver(proxyAddress, artifactPath, wallet) {
    try {
        const transceiverInterface = createTransceiverInterface(proxyAddress, artifactPath);
        const transceiverContract = transceiverInterface.connect(wallet);

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
 */
async function transferPauserCapability(proxyAddress, artifactPath, wallet, pauserAddress) {
    if (pauserAddress && isAddress(pauserAddress)) {
        try {
            const transceiverInterface = createTransceiverInterface(proxyAddress, artifactPath);
            const transceiverContract = transceiverInterface.connect(wallet);

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
 * Orchestrates the deployment of AxelarTransceiver and proxy.
 */
async function deployTransceiverContracts(config, chain, options) {
    // Create wallet for contract interactions
    const { ethers } = require('hardhat');
    const { Wallet, getDefaultProvider } = ethers;
    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet(options.privateKey, provider);

    // Deploy AxelarTransceiver implementation
    const implementationContract = await deployAxelarTransceiver(config, chain, options);
    if (!implementationContract) {
        return; // User cancelled or predictOnly mode
    }

    // Deploy ERC1967Proxy for AxelarTransceiver
    const proxyContract = await deployTransceiverProxy(config, chain, options, implementationContract.address);
    if (!proxyContract) {
        return; // User cancelled or predictOnly mode
    }

    // Initialize the contract if needed
    await initializeTransceiver(proxyContract.address, options.artifactPath, wallet);

    // Transfer pauser capability if provided
    await transferPauserCapability(proxyContract.address, options.artifactPath, wallet, options.pauserAddress);

    saveConfig(config, options.env);
}

/**
 * Main entry point for the deploy-transceiver script.
 */
async function main(options) {
    await mainProcessor(options, deployTransceiverContracts);
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

    program.addOption(new Option('--gmpManager <gmpManager>', 'GMP Manager address').makeOptionMandatory(true).env('GMP_MANAGER'));

    program.addOption(new Option('--pauserAddress <pauserAddress>', 'Address to transfer pauser capability to').env('PAUSER_ADDRESS'));

    program.addOption(
        new Option('--transceiverSalt <transceiverSalt>', 'deployment salt to use for AxelarTransceiver deployment').env(
            'TRANSCEIVER_SALT',
        ),
    );

    program.addOption(new Option('--proxySalt <proxySalt>', 'deployment salt to use for ERC1967Proxy deployment').env('PROXY_SALT'));

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
}
