const { Command, Option } = require('commander');
const { Transaction } = require('@mysten/sui/transactions');
// const { bcs } = require('@mysten/sui.js/bcs');

const { printInfo } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

// async function callContract(keypair, client, config, chain, contractId, functionName, functionArgs, options) {
//     if (!chain.contracts.operators) {
//         throw new Error('Operators package not found.');
//     }

//     const operatorsConfig = chain.contracts.operators;
//     const walletAddress = keypair.getPublicKey().toString();

//     const tx = new TransactionBlock();

//     let borrowedCap = null;

//     if (options.capId) {
//         [borrowedCap] = tx.moveCall({
//             target: `${operatorsConfig.address}::operators::borrow_cap`,
//             arguments: [
//                 tx.object(operatorsConfig.objects.operators),
//                 tx.object(operatorsConfig.objects.operator_caps[walletAddress]),
//                 tx.pure(options.capId),
//             ],
//         });
//     }

//     const callArgs = [...functionArgs];

//     if (options.capIndex !== undefined && borrowedCap) {
//         callArgs.splice(options.capIndex, 0, borrowedCap);
//     }

//     tx.moveCall({
//         target: `${contractId}::${functionName}`,
//         arguments: callArgs,
//     });

//     await broadcast(client, keypair, tx);

//     printInfo('Contract called successfully');
// }

// async function collectGas(keypair, client, config, chain, args, options) {
//     if (!chain.contracts.gas_service) {
//         throw new Error('Gas service package not found.');
//     }

//     const gasServiceConfig = chain.contracts.gas_service;
//     const [receiver, amount] = args;

//     await callContract(
//         keypair,
//         client,
//         config,
//         chain,
//         gasServiceConfig.address,
//         'gas_service::collect_gas',
//         [gasServiceConfig.objects.gas_service, receiver, amount],
//         {
//             ...options,
//             capIndex: 0,
//         },
//     );

//     printInfo('Gas collected successfully');
// }

// async function refundGas(keypair, client, config, chain, args, options) {
//     if (!chain.contracts.gas_service) {
//         throw new Error('Gas service package not found.');
//     }

//     const gasServiceConfig = chain.contracts.gas_service;
//     const [messageId, receiver, amount] = args;

//     await callContract(
//         keypair,
//         client,
//         config,
//         chain,
//         gasServiceConfig.address,
//         'gas_service::refund',
//         [gasServiceConfig.objects.gas_service, bcs.string().serialize(messageId).toBytes(), receiver, amount],
//         {
//             ...options,
//             capIndex: 0,
//         },
//     );

//     printInfo('Gas refunded successfully');
// }

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
    const config = loadSuiConfig(options.env);

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

    // const collectGasCmd = program
    //     .command('collect-gas <receiver> <amount>')
    //     .description('Collect gas from the gas service')
    //     .addOption(new Option('--capId <capId>', 'ID of the GasCollectorCap to borrow'))
    //     .action((receiver, amount, options) => mainProcessor(collectGas, [receiver, amount], options));

    // const refundGasCmd = program
    //     .command('refund-gas <messageId> <receiver> <amount>')
    //     .description('Refund gas from the gas service')
    //     .addOption(new Option('--capId <capId>', 'ID of the GasCollectorCap to borrow'))
    //     .action((messageId, receiver, amount, options) => mainProcessor(refundGas, [messageId, receiver, amount], options));

    program.addCommand(addCmd);
    program.addCommand(removeCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
