import { Command } from 'commander';
import { Contract, Wallet, getDefaultProvider, utils } from 'ethers';

import { addOptionsToCommands, prompt as promptUser } from '../common';
import { getContractJSON, getGasOptions, mainProcessor, printError, printInfo, printWalletInfo, printWarn } from './utils';

// eslint-disable-next-line @typescript-eslint/no-require-imports
const { addEvmOptions } = require('./cli-utils');

// Type definitions
interface ChainConfig {
    name: string;
    rpc: string;
    contracts?: {
        MonadAxelarTransceiver?: {
            address?: string;
            pauser?: string;
            owner?: string;
        };
    };
    confirmations?: number;
}

interface Options {
    env: string;
    artifactPath: string;
    privateKey: string;
    args: string[];
    yes?: boolean;
    chainNames?: string;
    skipChains?: string;
    startFromChain?: string;
    parallel?: boolean;
    saveChainSeparately?: boolean;
    gasOptions?: string;
    verify?: boolean;
    contractName?: string;
    deployMethod?: string;
    salt?: string;
    skipExisting?: boolean;
    upgrade?: boolean;
    predictOnly?: boolean;
}

interface GasOptions {
    gasLimit?: number;
    gasPrice?: string;
    maxFeePerGas?: string;
    maxPriorityFeePerGas?: string;
}

interface TransactionReceipt {
    blockNumber: number;
    hash: string;
}

interface TransceiverContract extends InstanceType<typeof Contract> {
    initialize: (options?: GasOptions) => Promise<{ hash: string; wait: () => Promise<TransactionReceipt> }>;
    pauser: () => Promise<string>;
    owner: () => Promise<string>;
    transferPauserCapability: (address: string, options?: GasOptions) => Promise<{ hash: string; wait: () => Promise<TransactionReceipt> }>;
    setAxelarChainId: (
        chainId: number,
        chainName: string,
        transceiverAddress: string,
        options?: GasOptions,
    ) => Promise<{ hash: string; wait: () => Promise<TransactionReceipt> }>;
}

async function initializeTransceiver(
    proxyAddress: string,
    artifactPath: string,
    wallet: InstanceType<typeof Wallet>,
    chain: ChainConfig,
    options: Options,
): Promise<void> {
    try {
        await printWalletInfo(wallet);

        const transceiverJson = getContractJSON('AxelarTransceiver', artifactPath);

        const transceiverContract = new Contract(proxyAddress, transceiverJson.abi, wallet) as TransceiverContract;

        printInfo('Transceiver contract address', proxyAddress);

        // Check if initialize function exists
        if (!transceiverContract.initialize) {
            throw new Error('initialize function not found in contract ABI');
        }

        printInfo('Initializing MonadAxelarTransceiver...');

        const gasOptions = await getGasOptions(chain, options, 'MonadAxelarTransceiver');

        if (promptUser(`Proceed with MonadAxelarTransceiver initialization on ${chain.name}?`, options.yes)) {
            return;
        }

        // Call initialize with ETH value since it's payable
        const initTx = await transceiverContract.initialize({
            ...gasOptions,
        });
        printInfo('Transaction hash', initTx.hash);
        printInfo('Waiting for transaction confirmation...');

        const receipt = (await initTx.wait()) as TransactionReceipt;
        printInfo('Transaction confirmed in block', receipt.blockNumber.toString());
        printInfo('MonadAxelarTransceiver initialized successfully');

        // Read addresses from contract state after initialization
        await readInitializationState(transceiverContract, receipt, wallet, chain, options);
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);

        if (
            errorMessage.includes('already initialized') ||
            errorMessage.includes('InvalidInitialization') ||
            errorMessage.includes('execution reverted')
        ) {
            printInfo('MonadAxelarTransceiver is already initialized');
        } else {
            printError('Failed to initialize transceiver', errorMessage);
        }
    }
}

async function readInitializationState(
    transceiverContract: TransceiverContract,
    receipt: TransactionReceipt,
    wallet: InstanceType<typeof Wallet>,
    chain: ChainConfig,
    options: Options,
): Promise<void> {
    try {
        const pauser = await transceiverContract.pauser();
        const owner = await transceiverContract.owner();

        printInfo('Pauser', pauser);
        printInfo('Owner', owner);

        if (!chain.contracts) {
            chain.contracts = {};
        }
        if (!chain.contracts.MonadAxelarTransceiver) {
            chain.contracts.MonadAxelarTransceiver = {};
        }

        chain.contracts.MonadAxelarTransceiver.pauser = pauser;
        chain.contracts.MonadAxelarTransceiver.owner = owner;
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        printError('Failed to read initialization state:', errorMessage);
        throw error;
    }
}

async function transferPauserCapability(
    proxyAddress: string,
    artifactPath: string,
    wallet: InstanceType<typeof Wallet>,
    pauserAddress: string,
    chain: ChainConfig,
    options: Options,
): Promise<void> {
    if (!pauserAddress || !utils.isAddress(pauserAddress)) {
        throw new Error(`Invalid pauser address: ${pauserAddress}`);
    }
    try {
        const transceiverJson = getContractJSON('AxelarTransceiver', artifactPath);
        const transceiverContract = new Contract(proxyAddress, transceiverJson.abi, wallet) as TransceiverContract;
        printInfo(`Transferring pauser capability to ${pauserAddress}...`);

        if (promptUser(`Proceed with transferring pauser capability to ${pauserAddress}?`, options.yes)) {
            return;
        }

        const gasOptions = await getGasOptions(chain, options, 'MonadAxelarTransceiver');

        const transferTx = await transceiverContract.transferPauserCapability(pauserAddress, {
            ...gasOptions,
        });
        printInfo('Transaction hash', transferTx.hash);
        printInfo('Waiting for transaction confirmation...');

        const receipt = (await transferTx.wait()) as TransactionReceipt;
        printInfo('Pauser capability transferred successfully');

        await readInitializationState(transceiverContract, receipt, wallet, chain, options);
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        if (errorMessage.includes('OwnableUnauthorizedAccount') || errorMessage.includes('CallerNotNttManager')) {
            printError('Insufficient permissions to transfer pauser capability');
        } else {
            printWarn('Could not transfer pauser capability:', errorMessage);
        }
        throw error;
    }
}

async function setAxelarChainId(
    proxyAddress: string,
    artifactPath: string,
    wallet: InstanceType<typeof Wallet>,
    chainId: number,
    chainName: string,
    transceiverAddress: string,
    chain: ChainConfig,
    options: Options,
): Promise<void> {
    if (!chainId || chainId <= 0) {
        throw new Error(`Invalid chain ID: ${chainId}`);
    }
    if (!chainName || chainName.trim() === '') {
        throw new Error(`Invalid chain name: ${chainName}`);
    }
    if (!transceiverAddress || transceiverAddress.trim() === '') {
        throw new Error(`Invalid transceiver address: ${transceiverAddress}`);
    }

    try {
        const transceiverJson = getContractJSON('AxelarTransceiver', artifactPath);
        const transceiverContract = new Contract(proxyAddress, transceiverJson.abi, wallet) as TransceiverContract;

        printInfo(`Setting Axelar chain ID mapping:`);
        printInfo(`  Wormhole Chain ID: ${chainId}`);
        printInfo(`  Axelar Chain Name: ${chainName}`);
        printInfo(`  Transceiver Address: ${transceiverAddress}`);

        if (promptUser(`Proceed with setting Axelar chain ID mapping?`, options.yes)) {
            return;
        }

        const gasOptions = await getGasOptions(chain, options, 'MonadAxelarTransceiver');

        const setChainIdTx = await transceiverContract.setAxelarChainId(chainId, chainName, transceiverAddress, {
            ...gasOptions,
        });
        printInfo('Transaction hash', setChainIdTx.hash);
        printInfo('Waiting for transaction confirmation...');

        await setChainIdTx.wait();
        printInfo('Axelar chain ID mapping set successfully');
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        if (errorMessage.includes('OwnableUnauthorizedAccount')) {
            printError('Insufficient permissions to set Axelar chain ID mapping');
        } else if (errorMessage.includes('ChainIdAlreadySet')) {
            printWarn('Chain ID is already set:', errorMessage);
        } else if (errorMessage.includes('AxelarChainIdAlreadySet')) {
            printWarn('Axelar chain ID is already set:', errorMessage);
        } else if (errorMessage.includes('InvalidChainIdParams')) {
            printError('Invalid chain ID parameters provided');
        } else {
            printError('Failed to set Axelar chain ID mapping:', errorMessage);
        }
        throw error;
    }
}

async function processCommand(_axelar, chain: ChainConfig, action: string, options: Options): Promise<void> {
    const { env, artifactPath, privateKey, args } = options;

    if (!artifactPath) {
        throw new Error('--artifactPath is required. Please provide the path to the compiled artifacts.');
    }

    if (!chain.contracts?.MonadAxelarTransceiver?.address) {
        printError('Chain contracts:', JSON.stringify(chain.contracts, null, 2));
        throw new Error('MonadAxelarTransceiver address not found in configuration');
    }

    const transceiverAddress = chain.contracts.MonadAxelarTransceiver.address;
    printInfo('Found transceiver address:', transceiverAddress);

    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet(privateKey, provider);

    printInfo(`Processing MonadAxelarTransceiver operation: ${action} for chain: ${chain.name}`);
    printInfo(`Transceiver address: ${transceiverAddress}`);

    switch (action) {
        case 'initialize': {
            await initializeTransceiver(transceiverAddress, artifactPath, wallet, chain, options);
            break;
        }

        case 'transfer-pauser': {
            const [pauserAddress] = args;
            if (!pauserAddress) {
                throw new Error('Pauser address is required for transfer-pauser command');
            }
            await transferPauserCapability(transceiverAddress, artifactPath, wallet, pauserAddress, chain, options);
            break;
        }

        case 'set-axelar-chain-id': {
            const [chainIdStr, chainName, targetTransceiverAddress] = args;
            if (!chainIdStr || !chainName || !targetTransceiverAddress) {
                throw new Error('chainId, chainName, and targetTransceiverAddress are required for set-axelar-chain-id command');
            }
            const chainId = parseInt(chainIdStr, 10);
            await setAxelarChainId(transceiverAddress, artifactPath, wallet, chainId, chainName, targetTransceiverAddress, chain, options);
            break;
        }

        default:
            throw new Error(`Unknown action: ${action}`);
    }
}

async function main(action: string, args: string[], options: Options): Promise<Record<string, unknown>> {
    options.args = args;
    return mainProcessor(options, (_axelar, chain: ChainConfig, _chains, options: Options) =>
        processCommand(_axelar, chain, action, options),
    ) as Promise<Record<string, unknown>>;
}

if (require.main === module) {
    const program = new Command();
    program.name('axelar-transceiver').description('Manage MonadAxelarTransceiver operations');

    program
        .command('initialize')
        .description('Initialize the MonadAxelarTransceiver contract')
        .action((options: Options, cmd: InstanceType<typeof Command>) => {
            main(cmd.name(), [], options);
        });

    program
        .command('transfer-pauser')
        .description('Transfer pauser capability to a new address')
        .argument('<pauser-address>', 'Address to transfer pauser capability to')
        .action((pauserAddress: string, options: Options, cmd: InstanceType<typeof Command>) => {
            main(cmd.name(), [pauserAddress], options);
        });

    program
        .command('set-axelar-chain-id')
        .description('Set Axelar chain ID mapping for cross-chain communication')
        .argument('<chain-id>', 'Wormhole chain ID for the target chain')
        .argument('<chain-name>', 'Axelar chain name for the target chain')
        .argument('<transceiver-address>', 'Address of the transceiver on the target chain')
        .action((chainId: string, chainName: string, transceiverAddress: string, options: Options, cmd: InstanceType<typeof Command>) => {
            main(cmd.name(), [chainId, chainName, transceiverAddress], options);
        });

    addOptionsToCommands(program, addEvmOptions, {
        artifactPath: true,
        contractName: false,
        ignoreChainNames: false,
        ignorePrivateKey: false,
    });

    program.parse();
}

module.exports = { processTransceiverOperations: processCommand };
