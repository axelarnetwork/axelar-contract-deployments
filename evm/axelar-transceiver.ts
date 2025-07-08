'use strict';

/**
 * @fileoverview AxelarTransceiver Post-Deployment Operations Script
 *
 * This script handles post-deployment operations for AxelarTransceiver contracts:
 * - Contract initialization
 * - Pauser capability transfer
 *
 * Usage: ts-node evm/axelar-transceiver.ts -e testnet -n ethereum --initialize --pauserAddress 0x...
 */
import { Command, Option } from 'commander';
import { Contract, Wallet, ethers, getDefaultProvider } from 'ethers';

import { getContractJSON, mainProcessor, printError, printInfo, printWarn, saveConfig } from './utils';

// Type definitions
interface ChainConfig {
    rpc: string;
    contracts?: {
        AxelarTransceiver?: {
            address: string;
        };
    };
}

interface Config {
    chains: Record<string, ChainConfig>;
    AxelarTransceiver?: {
        address: string;
    };
}

interface TransceiverOptions {
    privateKey: string;
    artifactPath: string;
    env: string;
    initialize?: boolean;
    pauserAddress?: string;
}

interface TransceiverInterface {
    initialize(): Promise<ethers.ContractTransaction>;
    transferPauserCapability(newPauser: string): Promise<ethers.ContractTransaction>;
    connect(wallet: Wallet): TransceiverInterface;
}

/**
 * Validates that the AxelarTransceiver contract exists in the configuration.
 * @throws {Error} If the AxelarTransceiver address is missing or invalid
 */
function validateTransceiverConfig(config: Config): void {
    if (!config.AxelarTransceiver?.address) {
        throw new Error('AxelarTransceiver address not found in configuration');
    }
    if (!ethers.utils.isAddress(config.AxelarTransceiver.address)) {
        throw new Error(`Invalid AxelarTransceiver address: ${config.AxelarTransceiver.address}`);
    }
}

/**
 * Validates the provided options for the transceiver operations.
 * @param options - The options to validate
 * @throws {Error} If required options are missing or invalid
 */
function validateOptions(options: TransceiverOptions): void {
    if (!options.privateKey) {
        throw new Error('Private key is required');
    }
    if (!options.artifactPath) {
        throw new Error('Artifact path is required');
    }
    if (!options.env) {
        throw new Error('Environment is required');
    }
    if (options.pauserAddress && !ethers.utils.isAddress(options.pauserAddress)) {
        throw new Error(`Invalid pauser address: ${options.pauserAddress}`);
    }
}

/**
 * Creates an AxelarTransceiver interface instance using the proxy address.
 */
function createTransceiverInterface(proxyAddress: string, artifactPath: string): TransceiverInterface {
    const transceiverJson = getContractJSON('AxelarTransceiver', artifactPath);

    // Create a contract instance with AxelarTransceiver ABI but proxy address
    return new Contract(proxyAddress, transceiverJson.abi, null) as unknown as TransceiverInterface;
}

/**
 * Initializes the AxelarTransceiver contract if not already initialized.
 */
async function initializeTransceiver(proxyAddress: string, artifactPath: string, wallet: Wallet): Promise<void> {
    try {
        const transceiverInterface = createTransceiverInterface(proxyAddress, artifactPath);
        const transceiverContract = transceiverInterface.connect(wallet);

        printInfo('Initializing AxelarTransceiver...');
        const initTx = await transceiverContract.initialize();
        await initTx.wait();
        printInfo('AxelarTransceiver initialized successfully');
    } catch (error: unknown) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        if (
            errorMessage.includes('already initialized') ||
            errorMessage.includes('InvalidInitialization') ||
            errorMessage.includes('execution reverted')
        ) {
            printInfo('AxelarTransceiver is already initialized');
        } else {
            printWarn('Failed to initialize transceiver:', errorMessage);
            throw error; // Re-throw to allow caller to handle
        }
    }
}

/**
 * Transfers pauser capability to the specified address.
 */
async function transferPauserCapability(proxyAddress: string, artifactPath: string, wallet: Wallet, pauserAddress: string): Promise<void> {
    if (!pauserAddress || !ethers.utils.isAddress(pauserAddress)) {
        throw new Error(`Invalid pauser address: ${pauserAddress}`);
    }

    try {
        const transceiverInterface = createTransceiverInterface(proxyAddress, artifactPath);
        const transceiverContract = transceiverInterface.connect(wallet);

        printInfo(`Transferring pauser capability to ${pauserAddress}...`);

        const transferTx = await transceiverContract.transferPauserCapability(pauserAddress);
        await transferTx.wait();
        printInfo('Pauser capability transferred successfully');
    } catch (error: unknown) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        if (errorMessage.includes('OwnableUnauthorizedAccount') || errorMessage.includes('CallerNotNttManager')) {
            printError('Insufficient permissions to transfer pauser capability');
        } else {
            printWarn('Could not transfer pauser capability:', errorMessage);
        }
        throw error; // Re-throw to allow caller to handle
    }
}

/**
 * Orchestrates post-deployment operations for AxelarTransceiver.
 */
async function processCommand(config: Config, chain: ChainConfig, options: TransceiverOptions): Promise<void> {
    try {
        // Validate configuration and options
        validateTransceiverConfig(config);
        validateOptions(options);

        // Create wallet for contract interactions
        const provider = getDefaultProvider(chain.rpc);
        const wallet = new Wallet(options.privateKey, provider);

        if (options.initialize) {
            await initializeTransceiver(config.AxelarTransceiver!.address, options.artifactPath, wallet);
        }

        if (options.pauserAddress) {
            await transferPauserCapability(config.AxelarTransceiver!.address, options.artifactPath, wallet, options.pauserAddress);
        }

        saveConfig(config, options.env);
    } catch (error: unknown) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        printError(`Failed to process transceiver operations: ${errorMessage}`);
        throw error;
    }
}

async function main(options: TransceiverOptions): Promise<void> {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('axelar-transceiver').description('AxelarTransceiver post-deployment operations (initialization, pauser transfer)');

    // Add basic EVM options
    program.addOption(new Option('-e, --env <env>', 'environment').makeOptionMandatory(true));
    program.addOption(new Option('-n, --network <network>', 'network name').makeOptionMandatory(true));
    program.addOption(new Option('--privateKey <privateKey>', 'private key').env('PRIVATE_KEY').makeOptionMandatory(true));
    program.addOption(new Option('--artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true));
    program.addOption(new Option('--skipExisting', 'skip if already deployed'));
    program.addOption(new Option('--predictOnly', 'only predict deployment address'));

    program.addOption(new Option('--pauserAddress <pauserAddress>', 'Address to transfer pauser capability to').env('PAUSER_ADDRESS'));

    program.addOption(new Option('--initialize', 'Initialize the transceiver').default(false));

    program.action(async (options: TransceiverOptions) => {
        await main(options);
    });

    program.parse();
}
