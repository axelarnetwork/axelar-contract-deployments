const { Command, Option } = require('commander');
const { Transaction } = require('@mysten/sui/transactions');
const { printInfo, printError, loadConfig } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    parseSuiUnitAmount,
    getWallet,
    printWalletInfo,
    broadcast,
    findOwnedObjectId,
} = require('./utils');

function operatorMoveCall(contractConfig, gasServiceConfig, operatorCapId, tx, moveCall) {
    const operatorId = contractConfig.objects.Operators;
    const gasCollectorCapId = gasServiceConfig.objects.GasCollectorCap;

    const [cap, loanedCap] = tx.moveCall({
        target: `${contractConfig.address}::operators::loan_cap`,
        arguments: [tx.object(operatorId), tx.object(operatorCapId), tx.object(gasCollectorCapId)],
        typeArguments: [`${gasServiceConfig.address}::gas_service::GasCollectorCap`],
    });

    moveCall(cap);

    tx.moveCall({
        target: `${contractConfig.address}::operators::restore_cap`,
        arguments: [tx.object(operatorId), tx.object(operatorCapId), tx.object(gasCollectorCapId), cap, loanedCap],
        typeArguments: [`${gasServiceConfig.address}::gas_service::GasCollectorCap`],
    });

    return tx;
}

async function collectGas(keypair, client, gasServiceConfig, contractConfig, args, options) {
    const [amount] = args;
    const receiver = options.receiver || keypair.toSuiAddress();

    const operatorCapId = await findOwnedObjectId(client, keypair.toSuiAddress(), `${contractConfig.address}::operators::OperatorCap`);
    const tx = new Transaction();

    operatorMoveCall(contractConfig, gasServiceConfig, operatorCapId, tx, (cap) => {
        tx.moveCall({
            target: `${gasServiceConfig.address}::gas_service::collect_gas`,
            arguments: [tx.object(gasServiceConfig.objects.GasService), cap, tx.pure.address(receiver), tx.pure.u64(amount)],
        });
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas collected', receipt.digest);
}

async function refund(keypair, client, gasServiceConfig, contractConfig, args, options) {
    const [messageId] = args;
    const amount = options.amount;
    const receiver = options.receiver || keypair.toSuiAddress();
    const operatorCapId = await findOwnedObjectId(client, keypair.toSuiAddress(), `${contractConfig.address}::operators::OperatorCap`);

    const tx = new Transaction();

    operatorMoveCall(contractConfig, gasServiceConfig, operatorCapId, tx, (cap) => {
        tx.moveCall({
            target: `${gasServiceConfig.address}::gas_service::refund`,
            arguments: [
                tx.object(gasServiceConfig.objects.GasService),
                cap,
                tx.pure.string(messageId),
                tx.pure.address(receiver),
                tx.pure.u64(amount),
            ],
        });
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas refunded', receipt.digest);
}

async function storeCap(keypair, client, gasServiceConfig, contractConfig, args, options) {
    const [capId] = args;
    const gasCollectorCapId = capId || gasServiceConfig.objects.GasCollectorCap;
    const ownerCapId = contractConfig.objects.OwnerCap;
    const operatorId = contractConfig.objects.Operators;

    const tx = new Transaction();

    tx.moveCall({
        target: `${contractConfig.address}::operators::store_cap`,
        arguments: [tx.object(operatorId), tx.object(ownerCapId), tx.object(gasCollectorCapId)],
        typeArguments: [`${gasServiceConfig.address}::gas_service::GasCollectorCap`],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Capability stored', receipt.digest);
}

async function addOperator(keypair, client, gasServiceConfig, contractConfig, args, options) {
    const [newOperatorAddress] = args;

    const operatorsObjectId = contractConfig.objects.Operators;
    const ownerCapObjectId = options.ownerCapId || contractConfig.objects.OwnerCap;

    const tx = new Transaction();

    tx.moveCall({
        target: `${contractConfig.address}::operators::add_operator`,
        arguments: [tx.object(operatorsObjectId), tx.object(ownerCapObjectId), tx.pure.address(newOperatorAddress)],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Operator Added', receipt.digest);
}

async function removeCap(keypair, client, gasServiceConfig, contractConfig, args, options) {
    const [capId] = args;

    const gasServiceAddress = gasServiceConfig.address;
    const operatorsObjectId = contractConfig.objects.Operators;
    const ownerCapObjectId = options.ownerCapId || contractConfig.objects.OwnerCap;
    const capReceiver = options.receiver || keypair.toSuiAddress();

    const tx = new Transaction();

    const cap = tx.moveCall({
        target: `${contractConfig.address}::operators::remove_cap`,
        arguments: [tx.object(operatorsObjectId), tx.object(ownerCapObjectId), tx.object(capId)],
        typeArguments: [`${gasServiceAddress}::gas_service::GasCollectorCap`],
    });

    tx.transferObjects([cap], capReceiver);

    try {
        const receipt = await broadcast(client, keypair, tx);

        printInfo('Capability Removed', receipt.digest);
    } catch (e) {
        printError('RemoveCap Error', e.message);
    }
}

async function removeOperator(keypair, client, gasServiceConfig, contractConfig, args, options) {
    const [operatorAddress] = args;

    const operatorsObjectId = contractConfig.objects.Operators;
    const ownerCapObjectId = options.ownerCapId || contractConfig.objects.OwnerCap;

    const tx = new Transaction();

    tx.moveCall({
        target: `${contractConfig.address}::operators::remove_operator`,
        arguments: [tx.object(operatorsObjectId), tx.object(ownerCapObjectId), tx.pure.address(operatorAddress)],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Operator Removed', receipt.digest);
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);

    const contractConfig = config.sui.contracts.Operators;
    const gasServiceConfig = config.sui.contracts.GasService;

    if (!contractConfig) {
        throw new Error('Operators package not found.');
    }

    if (!gasServiceConfig) {
        throw new Error('Gas service package not found.');
    }

    const [keypair, client] = getWallet(config.sui, options);
    await printWalletInfo(keypair, client, config.sui, options);
    await processor(keypair, client, gasServiceConfig, contractConfig, args, options);
}

if (require.main === module) {
    const program = new Command('operators');

    program.description('Operators contract operations.');

    program.addCommand(
        new Command('add')
            .command('add <newOperatorAddress>')
            .description('Add an operator')
            .addOption(new Option('--ownerCap <ownerCapId>', 'ID of the owner capability'))
            .action((newOperatorAddress, options) => mainProcessor(addOperator, [newOperatorAddress], options)),
    );

    program.addCommand(
        new Command('remove')
            .command('remove <operatorAddress>')
            .description('Remove an operator')
            .addOption(new Option('--ownerCap <ownerCapId>', 'ID of the owner capability'))
            .action((operatorAddress, options) => mainProcessor(removeOperator, [operatorAddress], options)),
    );

    program.addCommand(
        new Command('collectGas')
            .command('collectGas')
            .description('Collect gas from the gas service')
            .addOption(new Option('--receiver <receiver>', 'Address of the receiver'))
            .requiredOption('--amount <amount>', 'Amount to add gas', parseSuiUnitAmount)
            .action((options) => mainProcessor(collectGas, [options.amount], options)),
    );

    program.addCommand(
        new Command('storeCap')
            .command('storeCap')
            .description('Store a capability')
            .addOption(new Option('--capId <capId>', 'ID of the capability to store'))
            .action((options) => mainProcessor(storeCap, [], options)),
    );

    program.addCommand(
        new Command('removeCap')
            .command('removeCap <capId>')
            .description('Remove a capability')
            .addOption(new Option('--ownerCap <ownerCapId>', 'ID of the owner capability'))
            .addOption(new Option('--receiver <receiver>', 'The removed cap receiver address'))
            .action((capId, options) => mainProcessor(removeCap, [capId], options)),
    );

    program.addCommand(
        new Command('refund')
            .command('refund <messageId>')
            .description('Refund gas from the gas service')
            .addOption(new Option('--receiver <receiver>', 'Address of the receiver'))
            .requiredOption('--amount <amount>', 'Amount to refund', parseSuiUnitAmount)
            .action((messageId, options) => mainProcessor(refund, [messageId], options)),
    );

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
