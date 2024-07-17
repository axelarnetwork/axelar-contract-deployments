const { saveConfig, printInfo, printError } = require('../evm/utils');
const { Command } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { gasServiceStruct } = require('./types-utils');
const { loadSuiConfig, getBcsBytesByObjectId } = require('./utils');
const { ethers } = require('hardhat');
const { getUnitAmount, getFormattedAmount } = require('./amount-utils');
const {
    utils: { arrayify },
} = ethers;

const { addOptionsToCommands, addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');

async function payGas(keypair, client, gasServiceConfig, args, options) {
    const walletAddress = keypair.toSuiAddress();

    const gasServicePackageId = gasServiceConfig.address;

    const refundAddress = options.refundAddress || walletAddress;
    const params = options.params || '0x';

    const [amount, destinationChain, destinationAddress, channelId, payload] = args;

    const unitAmount = getUnitAmount(amount);

    const tx = new TransactionBlock();
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

    const refundAddress = options.refundAddress || walletAddress;
    const params = options.params || '0x';

    const [messageId, amount] = args;

    const unitAmount = getUnitAmount(amount);

    const tx = new TransactionBlock();
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

    const [amount] = args;
    const receiver = options.receiver || walletAddress;

    const unitAmount = getUnitAmount(amount);

    const bytes = await getBcsBytesByObjectId(client, gasServiceObjectId);
    const { balance: gasServiceBalance } = gasServiceStruct.parse(bytes);

    // Check if the gas service balance is sufficient
    if (gasServiceBalance < unitAmount) {
        printError('Insufficient gas service balance', `${getFormattedAmount(gasServiceBalance)} < ${getFormattedAmount(unitAmount)}`);
        return;
    }

    const tx = new TransactionBlock();

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

    const [messageId, amount] = args;
    const receiver = options.receiver || walletAddress;

    const unitAmount = getUnitAmount(amount);

    const bytes = await getBcsBytesByObjectId(client, gasServiceObjectId);
    const { balance: gasServiceBalance } = gasServiceStruct.parse(bytes);

    // Check if the gas service balance is sufficient
    if (gasServiceBalance < unitAmount) {
        printError('Insufficient gas service balance', `${getFormattedAmount(gasServiceBalance)} < ${getFormattedAmount(unitAmount)}`);
        return;
    }

    const tx = new TransactionBlock();
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
    const config = loadSuiConfig(options.env);
    await processor(command, config.sui, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('gas-service').description('Interact with the gas service contract.');

    const payGasCmd = new Command()
        .command('payGas <amount> <destinationChain> <destinationAddress> <channelId> <payload>')
        .description('Pay gas for the new contract call.')
        .option('--refundAddress <refundAddress>', 'Refund address. Default is the sender address.')
        .option('--params <params>', 'Params. Default is empty.')
        .action((amount, destinationChain, destinationAddress, channelId, payload, options) => {
            mainProcessor(options, [amount, destinationChain, destinationAddress, channelId, payload], processCommand, payGas);
        });

    const addGasCmd = new Command()
        .command('addGas <message_id> <amount>')
        .description('Add gas for the existing contract call.')
        .option('--refundAddress <refundAddress>', 'Refund address.')
        .option('--params <params>', 'Params. Default is empty.')
        .action((messageId, amount, options) => {
            mainProcessor(options, [messageId, amount], processCommand, addGas);
        });

    const collectGasCmd = new Command()
        .command('collectGas <amount>')
        .description('Collect gas from the gas service contract.')
        .option('--receiver <receiver>', 'Receiver address. Default is the sender address.')
        .action((amount, options) => {
            mainProcessor(options, [amount], processCommand, collectGas);
        });

    const refundCmd = new Command()
        .command('refund <messageId> <amount>')
        .description('Refund gas from the gas service contract.')
        .option('--receiver <receiver>', 'Receiver address. Default is the sender address.')
        .action((messageId, amount, options) => {
            mainProcessor(options, [messageId, amount], processCommand, refund);
        });

    program.addCommand(payGasCmd);
    program.addCommand(addGasCmd);
    program.addCommand(collectGasCmd);
    program.addCommand(refundCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
