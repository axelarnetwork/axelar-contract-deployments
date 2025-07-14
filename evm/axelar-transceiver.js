'use strict';

const { Command, Option } = require('commander');
const { addEvmOptions } = require('./cli-utils');
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

async function initializeTransceiver(proxyAddress, artifactPath, wallet, chain, options) {
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

async function transferPauserCapability(proxyAddress, artifactPath, wallet, pauserAddress, chain, options) {
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

        await transferTx.wait();
        printInfo('Pauser capability transferred successfully');
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

async function processTransceiverOperations(config, chain, options) {
    const { env, artifactPath, privateKey, initialize, pauserAddress } = options;

    if (!chain.contracts?.AxelarTransceiver?.address) {
        printError('Chain contracts:', JSON.stringify(chain.contracts, null, 2));
        throw new Error('AxelarTransceiver address not found in configuration');
    }

    const transceiverAddress = chain.contracts.AxelarTransceiver.address;
    printInfo('Found transceiver address:', transceiverAddress);

    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet(privateKey, provider);

    printInfo(`Processing AxelarTransceiver operations for chain: ${chain.name}`);
    printInfo(`Transceiver address: ${transceiverAddress}`);

    if (initialize) {
        await initializeTransceiver(transceiverAddress, artifactPath, wallet, chain, options);
    } else {
        printInfo('Initialize flag is false, skipping initialization');
    }

    if (pauserAddress) {
        await transferPauserCapability(transceiverAddress, artifactPath, wallet, pauserAddress, chain, options);
    }

    saveConfig(config, env);
}

async function main(options) {
    await mainProcessor(options, processTransceiverOperations);
}

if (require.main === module) {
    const program = new Command();
    program.name('axelar-transceiver').description('Manage AxelarTransceiver operations');
    addEvmOptions(program, {
        artifactPath: true,
        contractName: false,
        ignoreChainNames: false,
        ignorePrivateKey: false,
    });
    program.addOption(new Option('--initialize', 'Initialize the transceiver').default(false));
    program.addOption(new Option('--pauserAddress <pauserAddress>', 'Address to transfer pauser capability to').env('PAUSER_ADDRESS'));
    program.action((options) => {
        main(options);
    });
    program.parse();
}

module.exports = { processTransceiverOperations };
