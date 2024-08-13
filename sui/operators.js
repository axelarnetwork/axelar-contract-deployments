const { Command, Option } = require('commander');
const { Transaction } = require('@mysten/sui/transactions');
const { bcs } = require('@mysten/sui/bcs');
const { printInfo, loadConfig } = require('../common/utils');
const { operatorsStruct } = require('./types-utils');
const { addBaseOptions, addOptionsToCommands, parseSuiUnitAmount } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { getBcsBytesByObjectId, findOwnedObjectId } = require('./utils');

async function getGasCollectorCapId(client, gasServiceConfig, operatorsConfig) {
    const operatorId = operatorsConfig.objects.Operators;

    // Get and parse operator data
    const operatorBytes = await getBcsBytesByObjectId(client, operatorId);
    const parsedOperator = operatorsStruct.parse(operatorBytes);

    // Get the capabilities bag ID
    const bagId = parsedOperator.caps.id;

    // Find the GasCollectorCap bag ID
    const bagResult = await client.getDynamicFields({
        parentId: bagId,
        name: 'caps',
    });
    const gasCollectorBagId = bagResult.data.find(
        (cap) => cap.objectType === `${gasServiceConfig.address}::gas_service::GasCollectorCap`,
    )?.objectId;

    if (!gasCollectorBagId) {
        throw new Error('GasCollectorCap not found in the operator capabilities bag');
    }

    // Get the actual cap ID from the bag ID
    const gasCollectorCapObject = await client.getObject({
        id: gasCollectorBagId,
        options: {
            showContent: true,
        },
    });

    // Extract and return the gas collector cap ID
    const gasCollectorCapId = gasCollectorCapObject.data.content.fields.value.fields.id.id;
    return gasCollectorCapId;
}

async function collectGas(keypair, client, config, chain, args, options) {
    const [amount] = args;
    const receiver = options.receiver || keypair.toSuiAddress();
    const gasServiceConfig = config.sui.contracts.GasService;
    const operatorsConfig = config.sui.contracts.Operators;

    if (!gasServiceConfig) {
        throw new Error('Gas service package not found.');
    }

    if (!operatorsConfig) {
        throw new Error('Operators package not found.');
    }

    const operatorId = operatorsConfig.objects.Operators;
    const gasCollectorCapId = await getGasCollectorCapId(client, gasServiceConfig, operatorsConfig);
    const operatorCapId = await findOwnedObjectId(client, keypair.toSuiAddress(), `${operatorsConfig.address}::operators::OperatorCap`);

    const tx = new Transaction();

    const [cap, loanedCap] = tx.moveCall({
        target: `${operatorsConfig.address}::operators::loan_cap`,
        arguments: [tx.object(operatorId), tx.object(operatorCapId), tx.pure(bcs.Address.serialize(gasCollectorCapId).toBytes())],
        typeArguments: [`${gasServiceConfig.address}::gas_service::GasCollectorCap`],
    });

    tx.moveCall({
        target: `${gasServiceConfig.address}::gas_service::collect_gas`,
        arguments: [tx.object(gasServiceConfig.objects.GasService), cap, tx.pure.address(receiver), tx.pure.u64(amount)],
    });

    tx.moveCall({
        target: `${operatorsConfig.address}::operators::restore_cap`,
        arguments: [
            tx.object(operatorId),
            tx.object(operatorCapId),
            tx.pure(bcs.Address.serialize(gasCollectorCapId).toBytes()),
            cap,
            loanedCap,
        ],
        typeArguments: [`${gasServiceConfig.address}::gas_service::GasCollectorCap`],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas collected', receipt.digest);
}

async function refund(keypair, client, config, chain, args, options) {
    const [messageId] = args;
    const amount = options.amount;
    const receiver = options.receiver || keypair.toSuiAddress();

    const gasServiceConfig = config.sui.contracts.GasService;
    const operatorsConfig = chain;

    if (!gasServiceConfig) {
        throw new Error('Gas service package not found.');
    }

    if (!operatorsConfig) {
        throw new Error('Operators package not found.');
    }

    const operatorId = operatorsConfig.objects.Operators;
    const gasCollectorCapId = await getGasCollectorCapId(client, gasServiceConfig, operatorsConfig);
    const operatorCapId = await findOwnedObjectId(client, keypair.toSuiAddress(), `${operatorsConfig.address}::operators::OperatorCap`);

    const tx = new Transaction();

    const [cap, loanedCap] = tx.moveCall({
        target: `${operatorsConfig.address}::operators::loan_cap`,
        arguments: [tx.object(operatorId), tx.object(operatorCapId), tx.pure(bcs.Address.serialize(gasCollectorCapId).toBytes())],
        typeArguments: [`${gasServiceConfig.address}::gas_service::GasCollectorCap`],
    });

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

    tx.moveCall({
        target: `${operatorsConfig.address}::operators::restore_cap`,
        arguments: [
            tx.object(operatorId),
            tx.object(operatorCapId),
            tx.pure(bcs.Address.serialize(gasCollectorCapId).toBytes()),
            cap,
            loanedCap,
        ],
        typeArguments: [`${gasServiceConfig.address}::gas_service::GasCollectorCap`],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas refunded', receipt.digest);
}

async function storeCap(keypair, client, config, chain, args, options) {
    const [capId] = args;
    const gasCollectorCapConfig = config.sui.contracts.GasService;
    const gasCollectorCapId = capId || gasCollectorCapConfig.objects.GasCollectorCap;
    const operatorsConfig = config.sui.contracts.Operators;
    const ownerCapId = operatorsConfig.objects.OwnerCap;
    const operatorId = operatorsConfig.objects.Operators;

    const tx = new Transaction();

    tx.moveCall({
        target: `${operatorsConfig.address}::operators::store_cap`,
        arguments: [tx.object(operatorId), tx.object(ownerCapId), tx.object(gasCollectorCapId)],
        typeArguments: [`${gasCollectorCapConfig.address}::gas_service::GasCollectorCap`],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Capability stored', receipt.digest);
}

async function addOperator(keypair, client, config, operatorsConfig, args, options) {
    const [newOperatorAddress] = args;

    const operatorsObjectId = operatorsConfig.objects.Operators;
    const ownerCapObjectId = options.ownerCapId || operatorsConfig.objects.OwnerCap;

    const tx = new Transaction();

    tx.moveCall({
        target: `${operatorsConfig.address}::operators::add_operator`,
        arguments: [tx.object(operatorsObjectId), tx.object(ownerCapObjectId), tx.pure.address(newOperatorAddress)],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Operator Added', receipt.digest);
}

async function removeCap(keypair, client, config, operatorsConfig, args, options) {
    const [capId] = args;

    const gasServiceAddress = config.sui.contracts.GasService.address;
    const operatorsObjectId = operatorsConfig.objects.Operators;
    const ownerCapObjectId = options.ownerCapId || operatorsConfig.objects.OwnerCap;
    const capReceiver = options.receiver || keypair.toSuiAddress();

    const tx = new Transaction();

    const cap = tx.moveCall({
        target: `${operatorsConfig.address}::operators::remove_cap`,
        arguments: [tx.object(operatorsObjectId), tx.object(ownerCapObjectId), tx.object(capId)],
        typeArguments: [`${gasServiceAddress}::gas_service::GasCollectorCap`],
    });

    tx.transferObjects([cap], capReceiver);

    try {
        const receipt = await broadcast(client, keypair, tx);

        printInfo('Capability Removed', receipt.digest);
    } catch (e) {
        console.log('Error', e.message);
    }
}

async function removeOperator(keypair, client, config, operatorsConfig, args, options) {
    const [operatorAddress] = args;

    const operatorsObjectId = operatorsConfig.objects.Operators;
    const ownerCapObjectId = options.ownerCapId || operatorsConfig.objects.OwnerCap;

    const tx = new Transaction();

    tx.moveCall({
        target: `${operatorsConfig.address}::operators::remove_operator`,
        arguments: [tx.object(operatorsObjectId), tx.object(ownerCapObjectId), tx.pure.address(operatorAddress)],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Operator Removed', receipt.digest);
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);

    if (!config.sui.contracts.Operators) {
        throw new Error('Operators package not found.');
    }

    const [keypair, client] = getWallet(config.sui, options);
    await printWalletInfo(keypair, client, config.sui, options);

    await processor(keypair, client, config, config.sui.contracts.Operators, args, options);
}

if (require.main === module) {
    const program = new Command('operators');

    program.description('Operators contract operations.');

    const addCmd = new Command('add')
        .command('add <newOperatorAddress>')
        .description('Add an operator')
        .addOption(new Option('--ownerCap <ownerCapId>', 'ID of the owner capability'))
        .action((newOperatorAddress, options) => mainProcessor(addOperator, [newOperatorAddress], options));

    const removeCmd = new Command('remove')
        .command('remove <operatorAddress>')
        .description('Remove an operator')
        .addOption(new Option('--ownerCap <ownerCapId>', 'ID of the owner capability'))
        .action((operatorAddress, options) => mainProcessor(removeOperator, [operatorAddress], options));

    const collectGasCmd = new Command('collectGas')
        .command('collectGas')
        .description('Collect gas from the gas service')
        .addOption(new Option('--receiver <receiver>', 'Address of the receiver'))
        .requiredOption('--amount <amount>', 'Amount to add gas', parseSuiUnitAmount)
        .action((options) => mainProcessor(collectGas, [options.amount], options));

    const storeCapCmd = new Command('storeCap')
        .command('storeCap')
        .description('Store a capability')
        .addOption(new Option('--capId <capId>', 'ID of the capability to store'))
        .action((options) => mainProcessor(storeCap, [], options));

    const removeCapCmd = new Command('removeCap')
        .command('removeCap <capId>')
        .description('Remove a capability')
        .addOption(new Option('--ownerCap <ownerCapId>', 'ID of the owner capability'))
        .addOption(new Option('--receiver <receiver>', 'The removed cap receiver address'))
        .action((capId, options) => mainProcessor(removeCap, [capId], options));

    const refundCmd = new Command('refund')
        .command('refund <messageId>')
        .description('Refund gas from the gas service')
        .addOption(new Option('--receiver <receiver>', 'Address of the receiver'))
        .requiredOption('--amount <amount>', 'Amount to refund', parseSuiUnitAmount)
        .action((messageId, options) => mainProcessor(refund, [messageId], options));

    program.addCommand(addCmd);
    program.addCommand(removeCmd);
    program.addCommand(collectGasCmd);
    program.addCommand(storeCapCmd);
    program.addCommand(removeCapCmd);
    program.addCommand(refundCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
