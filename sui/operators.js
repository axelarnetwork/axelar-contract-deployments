const { Command, Option } = require('commander');
const { Transaction } = require('@mysten/sui/transactions');
// const { bcs } = require('@mysten/sui.js/bcs');

const { printInfo, loadConfig } = require('../common/utils');
const { operatorsStruct } = require('./types-utils');
const { addBaseOptions, addOptionsToCommands, parseSuiUnitAmount } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { getBcsBytesByObjectId } = require('./utils');

async function callContract(keypair, client, config, chain, contractId, functionName, functionArgs, options) {
    if (!config.sui.contracts.Operators) {
        throw new Error('Operators package not found.');
    }

    const operatorsConfig = config.sui.contracts.Operators;
    const walletAddress = keypair.toSuiAddress();

    const operatorBytes = await getBcsBytesByObjectId(client, operatorsConfig.objects.Operators);
    const parsedOperator = operatorsStruct.parse(operatorBytes);
    const bagId = parsedOperator.caps.id;
    console.log(parsedOperator);
    const bagResult = await client.getDynamicFields({
        parentId: bagId,
        name: 'caps',
    });
    console.log(bagResult);
    return;

    const tx = new Transaction();

    let borrowedCap = null;

    if (options.capId) {
        [borrowedCap] = tx.moveCall({
            target: `${operatorsConfig.address}::operators::borrow_cap`,
            arguments: [
                tx.object(operatorsConfig.objects.Operators),
                tx.object(operatorsConfig.objects.operator_caps[walletAddress]),
                tx.pure(options.capId),
            ],
        });
    }

    const callArgs = [...functionArgs];

    if (options.capIndex !== undefined && borrowedCap) {
        callArgs.splice(options.capIndex, 0, borrowedCap);
    }

    tx.moveCall({
        target: `${contractId}::${functionName}`,
        arguments: callArgs,
    });

    await broadcast(client, keypair, tx);

    printInfo('Contract called successfully');
}

// await processor(keypair, client, config, config.sui.contracts.Operators, args, options);
async function collectGas(keypair, client, config, chain, args, options) {
    if (!config.sui.contracts.GasService) {
        throw new Error('Gas service package not found.');
    }

    const gasServiceConfig = config.sui.contracts.GasService;
    const [receiver, amount] = args;

    await callContract(
        keypair,
        client,
        config,
        chain,
        gasServiceConfig.address,
        'GasService::collect_gas',
        [gasServiceConfig.objects.GasService, receiver, amount],
        {
            ...options,
            capIndex: 0,
        },
    );

    printInfo('Gas collected successfully');
}

// async function refundGas(keypair, client, config, chain, args, options) {
//     if (!chain.contracts.GasService) {
//         throw new Error('Gas service package not found.');
//     }

//     const gasServiceConfig = chain.contracts.GasService;
//     const [messageId, receiver, amount] = args;

//     await callContract(
//         keypair,
//         client,
//         config,
//         chain,
//         gasServiceConfig.address,
//         'GasService::refund',
//         [gasServiceConfig.objects.GasService, bcs.string().serialize(messageId).toBytes(), receiver, amount],
//         {
//             ...options,
//             capIndex: 0,
//         },
//     );

//     printInfo('Gas refunded successfully');
// }

async function storeCap(keypair, client, config, chain, args, options) {
    const [capId] = args;
    const gasCollectorCapId = capId || config.sui.contracts.GasService.objects.GasCollectorCap;

    const operatorsConfig = config.sui.contracts.Operators;
    const ownerCapId = operatorsConfig.objects.OwnerCap;
    const operatorId = operatorsConfig.objects.Operators;

    const tx = new Transaction();

    console.log('storeCap', capId, ownerCapId, operatorId);

    tx.moveCall({
        target: `${operatorsConfig.address}::operators::store_cap`,
        arguments: [tx.object(operatorId), tx.object(ownerCapId), tx.object(gasCollectorCapId)],
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

    // const callContractCmd = program
    //     .command('call-contract <contractId> <functionName> [functionArgs...]')
    //     .description('Call a contract with an optional borrowed capability')
    //     .addOption(new Option('--capId <capId>', 'ID of the capability to borrow'))
    //     .addOption(new Option('--capIndex <capIndex>', 'Index of the borrowed capability in the function arguments'))
    //     .action((contractId, functionName, functionArgs, options) => {
    //         options.capIndex = options.capIndex ? parseInt(options.capIndex, 10) : undefined;
    //         mainProcessor(callContract, [contractId, functionName, functionArgs], options);
    //     });

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
        .command('collectGas <receiver>')
        .description('Collect gas from the gas service')
        .requiredOption('--amount <amount>', 'Amount to add gas', parseSuiUnitAmount)
        .addOption(new Option('--capId <capId>', 'ID of the GasCollectorCap to borrow'))
        .action((receiver, options) => mainProcessor(collectGas, [receiver, options.amount], options));

    const storeCapCmd = new Command('storeCap')
        .command('storeCap')
        .description('Store a capability')
        .addOption(new Option('--capId <capId>', 'ID of the capability to store'))
        .action((options) => mainProcessor(storeCap, [], options));

    // const refundGasCmd = program
    //     .command('refund-gas <messageId> <receiver> <amount>')
    //     .description('Refund gas from the gas service')
    //     .addOption(new Option('--capId <capId>', 'ID of the GasCollectorCap to borrow'))
    //     .action((messageId, receiver, amount, options) => mainProcessor(refundGas, [messageId, receiver, amount], options));

    program.addCommand(addCmd);
    program.addCommand(removeCmd);
    program.addCommand(collectGasCmd);
    program.addCommand(storeCapCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
