'use strict';

const { Command } = require('commander');
const { addEvmOptions } = require('./cli-utils');
const { addOptionsToCommands } = require('../common');
const {
    getContractJSON,
    mainProcessor,
    printError,
    printInfo,
    printWarn,
    saveConfig,
    printWalletInfo,
    getGasOptions,
    prompt,
} = require('./utils');
const { Contract, Wallet, getDefaultProvider, utils } = require('ethers');

async function initializeTransceiver(proxyAddress, artifactPath, wallet, chain, options, config) {
    try {
        await printWalletInfo(wallet);

        const transceiverJson = getContractJSON('AxelarTransceiver', artifactPath);

        const transceiverContract = new Contract(proxyAddress, transceiverJson.abi, wallet);

        printInfo('Transceiver contract address', proxyAddress);

        // Check if initialize function exists
        if (!transceiverContract.initialize) {
            throw new Error('initialize function not found in contract ABI');
        }

        printInfo('Initializing AxelarTransceiver...');

        const gasOptions = await getGasOptions(chain, options, 'AxelarTransceiver');

        if (prompt(`Proceed with AxelarTransceiver initialization on ${chain.name}?`, options.yes)) {
            return;
        }

        // Call initialize with ETH value since it's payable
        const initTx = await transceiverContract.initialize({
            ...gasOptions,
        });
        printInfo('Transaction hash', initTx.hash);
        printInfo('Waiting for transaction confirmation...');

        const receipt = await initTx.wait();
        printInfo('Transaction confirmed in block', receipt.blockNumber);
        printInfo('AxelarTransceiver initialized successfully');

        // Read addresses from contract state after initialization
        await readInitializationState(transceiverContract, receipt, wallet, chain, options, config);
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);

        if (
            errorMessage.includes('already initialized') ||
            errorMessage.includes('InvalidInitialization') ||
            errorMessage.includes('execution reverted')
        ) {
            printInfo('AxelarTransceiver is already initialized');
        } else {
            printError('Failed to initialize transceiver', errorMessage);
        }
    }
}

async function readInitializationState(transceiverContract, receipt, wallet, chain, options, config) {
    try {
        const pauser = await transceiverContract.pauser();
        const owner = await transceiverContract.owner();

        printInfo('Pauser', pauser);
        printInfo('Owner', owner);

        if (!chain.contracts.AxelarTransceiver) {
            chain.contracts.AxelarTransceiver = {};
        }

        chain.contracts.AxelarTransceiver.pauser = pauser;
        chain.contracts.AxelarTransceiver.owner = owner;
        saveConfig(config, options.env);
    } catch (error) {
        printError('Failed to read initialization state:', error.message);
        throw error;
    }
}

async function transferPauserCapability(proxyAddress, artifactPath, wallet, pauserAddress, chain, options, config) {
    if (!pauserAddress || !utils.isAddress(pauserAddress)) {
        throw new Error(`Invalid pauser address: ${pauserAddress}`);
    }
    try {
        const transceiverJson = getContractJSON('AxelarTransceiver', artifactPath);
        const transceiverContract = new Contract(proxyAddress, transceiverJson.abi, wallet);
        printInfo(`Transferring pauser capability to ${pauserAddress}...`);

        if (prompt(`Proceed with transferring pauser capability to ${pauserAddress}?`, options.yes)) {
            return;
        }

        const gasOptions = await getGasOptions(chain, options, 'AxelarTransceiver');

        const transferTx = await transceiverContract.transferPauserCapability(pauserAddress, {
            ...gasOptions,
        });
        printInfo('Transaction hash', transferTx.hash);
        printInfo('Waiting for transaction confirmation...');

        const receipt = await transferTx.wait();
        printInfo('Pauser capability transferred successfully');

        await readInitializationState(transceiverContract, receipt, wallet, chain, options, config);
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

async function setAxelarChainId(proxyAddress, artifactPath, wallet, chainId, chainName, transceiverAddress, chain, options) {
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
        const transceiverContract = new Contract(proxyAddress, transceiverJson.abi, wallet);

        printInfo(`Setting Axelar chain ID mapping:`);
        printInfo(`  Wormhole Chain ID: ${chainId}`);
        printInfo(`  Axelar Chain Name: ${chainName}`);
        printInfo(`  Transceiver Address: ${transceiverAddress}`);

        if (prompt(`Proceed with setting Axelar chain ID mapping?`, options.yes)) {
            return;
        }

        const gasOptions = await getGasOptions(chain, options, 'AxelarTransceiver');

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

async function processCommand(config, chain, action, options) {
    const { env, artifactPath, privateKey, args } = options;

    if (!chain.contracts?.AxelarTransceiver?.address) {
        printError('Chain contracts:', JSON.stringify(chain.contracts, null, 2));
        throw new Error('AxelarTransceiver address not found in configuration');
    }

    const transceiverAddress = chain.contracts.AxelarTransceiver.address;
    printInfo('Found transceiver address:', transceiverAddress);

    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet(privateKey, provider);

    printInfo(`Processing AxelarTransceiver operation: ${action} for chain: ${chain.name}`);
    printInfo(`Transceiver address: ${transceiverAddress}`);

    switch (action) {
        case 'initialize': {
            await initializeTransceiver(transceiverAddress, artifactPath, wallet, chain, options, config);
            break;
        }

        case 'transfer-pauser': {
            const [pauserAddress] = args;
            if (!pauserAddress) {
                throw new Error('Pauser address is required for transfer-pauser command');
            }
            await transferPauserCapability(transceiverAddress, artifactPath, wallet, pauserAddress, chain, options, config);
            break;
        }

        case 'set-axelar-chain-id': {
            const [chainId, chainName, targetTransceiverAddress] = args;
            if (!chainId || !chainName || !targetTransceiverAddress) {
                throw new Error('chainId, chainName, and targetTransceiverAddress are required for set-axelar-chain-id command');
            }
            await setAxelarChainId(transceiverAddress, artifactPath, wallet, chainId, chainName, targetTransceiverAddress, chain, options);
            break;
        }

        default:
            throw new Error(`Unknown action: ${action}`);
    }

    saveConfig(config, env);
}

async function main(action, args, options) {
    options.args = args;
    return mainProcessor(options, (config, chain, options) => processCommand(config, chain, action, options));
}

if (require.main === module) {
    const program = new Command();
    program.name('axelar-transceiver').description('Manage AxelarTransceiver operations');

    program
        .command('initialize')
        .description('Initialize the AxelarTransceiver contract')
        .action((options, cmd) => {
            main(cmd.name(), [], options);
        });

    program
        .command('transfer-pauser')
        .description('Transfer pauser capability to a new address')
        .argument('<pauser-address>', 'Address to transfer pauser capability to')
        .action((pauserAddress, options, cmd) => {
            main(cmd.name(), [pauserAddress], options);
        });

    program
        .command('set-axelar-chain-id')
        .description('Set Axelar chain ID mapping for cross-chain communication')
        .argument('<chain-id>', 'Wormhole chain ID for the target chain')
        .argument('<chain-name>', 'Axelar chain name for the target chain')
        .argument('<transceiver-address>', 'Address of the transceiver on the target chain')
        .action((chainId, chainName, transceiverAddress, options, cmd) => {
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
