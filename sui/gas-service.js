const { Command } = require('commander');
const { Transaction } = require('@mysten/sui/transactions');
const { bcs } = require('@mysten/sui/bcs');
const { ethers } = require('hardhat');
const { bcsStructs } = require('@axelar-network/axelar-cgp-sui');
const {
    utils: { arrayify },
} = ethers;
const { saveConfig, loadConfig, printInfo, printError } = require('../common/utils');
const {
    getWallet,
    printWalletInfo,
    broadcast,
    getBcsBytesByObjectId,
    getFormattedAmount,
    addOptionsToCommands,
    addBaseOptions,
    parseSuiUnitAmount,
} = require('./utils');

async function payGas(keypair, client, gasServiceConfig, args, options) {
    const walletAddress = keypair.toSuiAddress();

    const gasServicePackageId = gasServiceConfig.address;

    const { params } = options;
    const refundAddress = options.refundAddress || walletAddress;

    const [destinationChain, destinationAddress, channelId, payload] = args;
    const unitAmount = options.amount;

    const tx = new Transaction();
    const [coin] = tx.splitCoins(tx.gas, [unitAmount]);

    tx.moveCall({
        target: `${gasServicePackageId}::gas_service::pay_gas`,
        arguments: [
            tx.object(gasServiceConfig.objects.GasService),
            coin, // Coin<SUI>
            tx.pure.address(channelId), // Channel address
            tx.pure(bcs.string().serialize(destinationChain).toBytes()), // Destination chain
            tx.pure(bcs.string().serialize(destinationAddress).toBytes()), // Destination address
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()), // Payload
            tx.pure.address(refundAddress), // Refund address
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(params)).toBytes()), // Params
        ],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas paid', receipt.digest);
}

async function addGas(keypair, client, gasServiceConfig, args, options) {
    const walletAddress = keypair.toSuiAddress();

    const gasServicePackageId = gasServiceConfig.address;

    const { params } = options;
    const refundAddress = options.refundAddress || walletAddress;

    const [messageId] = args;
    const unitAmount = options.amount;

    const tx = new Transaction();
    const [coin] = tx.splitCoins(tx.gas, [unitAmount]);

    tx.moveCall({
        target: `${gasServicePackageId}::gas_service::add_gas`,
        arguments: [
            tx.object(gasServiceConfig.objects.GasService),
            coin, // Coin<SUI>
            tx.pure(bcs.string().serialize(messageId).toBytes()), // Message ID for the contract call
            tx.pure.address(refundAddress), // Refund address
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(params)).toBytes()), // Params
        ],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas added', receipt.digest);
}

async function collectGas(keypair, client, gasServiceConfig, args, options) {
    const walletAddress = keypair.toSuiAddress();

    const gasServicePackageId = gasServiceConfig.address;
    const gasServiceObjectId = gasServiceConfig.objects.GasService;

    const unitAmount = options.amount;
    const receiver = options.receiver || walletAddress;

    const bytes = await getBcsBytesByObjectId(client, gasServiceObjectId);
    const { balance: gasServiceBalance } = bcsStructs.gasService.GasService.parse(bytes);

    // Check if the gas service balance is sufficient
    if (gasServiceBalance < unitAmount) {
        printError('Insufficient gas service balance', `${getFormattedAmount(gasServiceBalance)} < ${getFormattedAmount(unitAmount)}`);
        return;
    }

    const tx = new Transaction();

    tx.moveCall({
        target: `${gasServicePackageId}::gas_service::collect_gas`,
        arguments: [
            tx.object(gasServiceConfig.objects.GasService),
            tx.object(gasServiceConfig.objects.GasCollectorCap),
            tx.pure.address(receiver), // Receiver address
            tx.pure.u64(unitAmount), // Amount
        ],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas collected', receipt.digest);
}

async function refund(keypair, client, gasServiceConfig, args, options) {
    const walletAddress = keypair.toSuiAddress();

    const gasServicePackageId = gasServiceConfig.address;
    const gasServiceObjectId = gasServiceConfig.objects.GasService;

    const [messageId] = args;
    const unitAmount = options.amount;
    const receiver = options.receiver || walletAddress;

    const bytes = await getBcsBytesByObjectId(client, gasServiceObjectId);
    const { balance: gasServiceBalance } = bcsStructs.gasService.GasService.parse(bytes);

    // Check if the gas service balance is sufficient
    if (gasServiceBalance < unitAmount) {
        printError('Insufficient gas service balance', `${getFormattedAmount(gasServiceBalance)} < ${getFormattedAmount(unitAmount)}`);
        return;
    }

    const tx = new Transaction();
    tx.moveCall({
        target: `${gasServicePackageId}::gas_service::refund`,
        arguments: [
            tx.object(gasServiceConfig.objects.GasService),
            tx.object(gasServiceConfig.objects.GasCollectorCap),
            tx.pure(bcs.string().serialize(messageId).toBytes()), // Message ID for the contract call
            tx.pure.address(receiver), // Refund address
            tx.pure.u64(unitAmount), // Amount
        ],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas refunded', receipt.digest);
}

async function processCommand(command, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts.GasService) {
        throw new Error('GasService contract not found');
    }

    await command(keypair, client, chain.contracts.GasService, args, options);
}

async function mainProcessor(options, args, processor, command) {
    const config = loadConfig(options.env);
    await processor(command, config.sui, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('gas-service').description('Interact with the gas service contract.');

    const payGasCmd = new Command()
        .command('payGas <destinationChain> <destinationAddress> <channelId> <payload>')
        .description('Pay gas for the new contract call.')
        .option('--refundAddress <refundAddress>', 'Refund address. Default is the sender address.')
        .requiredOption('--amount <amount>', 'Amount to pay gas', parseSuiUnitAmount)
        .option('--params <params>', 'Params. Default is empty.', '0x')
        .action((destinationChain, destinationAddress, channelId, payload, options) => {
            mainProcessor(options, [destinationChain, destinationAddress, channelId, payload], processCommand, payGas);
        });

    const addGasCmd = new Command()
        .command('addGas <message_id>')
        .description('Add gas for the existing contract call.')
        .option('--refundAddress <refundAddress>', 'Refund address.')
        .requiredOption('--amount <amount>', 'Amount to add gas', parseSuiUnitAmount)
        .option('--params <params>', 'Params. Default is empty.')
        .action((messageId, options) => {
            mainProcessor(options, [messageId], processCommand, addGas);
        });

    const collectGasCmd = new Command()
        .command('collectGas')
        .description('Collect gas from the gas service contract.')
        .option('--receiver <receiver>', 'Receiver address. Default is the sender address.')
        .requiredOption('--amount <amount>', 'Amount to collect gas', parseSuiUnitAmount)
        .action((options) => {
            mainProcessor(options, [], processCommand, collectGas);
        });

    const refundCmd = new Command()
        .command('refund <messageId>')
        .description('Refund gas from the gas service contract.')
        .option('--receiver <receiver>', 'Receiver address. Default is the sender address.')
        .requiredOption('--amount <amount>', 'Amount to refund gas', parseSuiUnitAmount)
        .action((messageId, options) => {
            mainProcessor(options, [messageId], processCommand, refund);
        });

    program.addCommand(payGasCmd);
    program.addCommand(addGasCmd);
    program.addCommand(collectGasCmd);
    program.addCommand(refundCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
