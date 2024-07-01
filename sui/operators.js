const { Command, Option } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');

const { printInfo } = require('../evm/utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

async function callContract(keypair, client, config, chain, contractId, functionName, functionArgs, options) {
    if (!chain.contracts.operators) {
        throw new Error('Operators package not found.');
    }

    const operatorsConfig = chain.contracts.operators;
    const walletAddress = keypair.getPublicKey().toString();

    const tx = new TransactionBlock();

    let borrowedCap = null;

    if (options.capId) {
        [borrowedCap] = tx.moveCall({
            target: `${operatorsConfig.address}::operators::borrow_cap`,
            arguments: [
                tx.object(operatorsConfig.objects.operators),
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

async function collectGas(keypair, client, config, chain, args, options) {
    if (!chain.contracts.gas_service) {
        throw new Error('Gas service package not found.');
    }

    const gasServiceConfig = chain.contracts.gas_service;
    const [receiver, amount] = args;

    await callContract(
        keypair,
        client,
        config,
        chain,
        gasServiceConfig.address,
        'gas_service::collect_gas',
        [gasServiceConfig.objects.gas_service, receiver, amount],
        {
            ...options,
            capIndex: 0,
        },
    );

    printInfo('Gas collected successfully');
}

async function refundGas(keypair, client, config, chain, args, options) {
    if (!chain.contracts.gas_service) {
        throw new Error('Gas service package not found.');
    }

    const gasServiceConfig = chain.contracts.gas_service;
    const [messageId, receiver, amount] = args;

    await callContract(
        keypair,
        client,
        config,
        chain,
        gasServiceConfig.address,
        'gas_service::refund',
        [gasServiceConfig.objects.gas_service, bcs.string().serialize(messageId).toBytes(), receiver, amount],
        {
            ...options,
            capIndex: 0,
        },
    );

    printInfo('Gas refunded successfully');
}

async function mainProcessor(processor, args, options) {
    const config = loadSuiConfig(options.env);

    const [keypair, client] = getWallet(config.sui, options);
    await printWalletInfo(keypair, client, config.sui, options);

    await processor(keypair, client, config, config.sui, args, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('operators').description('Operators contract operations.');

    const callContractCmd = program
        .command('call-contract <contractId> <functionName> [functionArgs...]')
        .description('Call a contract with an optional borrowed capability')
        .addOption(new Option('--capId <capId>', 'ID of the capability to borrow'))
        .addOption(new Option('--capIndex <capIndex>', 'Index of the borrowed capability in the function arguments'))
        .action((contractId, functionName, functionArgs, options) => {
            options.capIndex = options.capIndex ? parseInt(options.capIndex, 10) : undefined;
            mainProcessor(callContract, [contractId, functionName, functionArgs], options);
        });

    const collectGasCmd = program
        .command('collect-gas <receiver> <amount>')
        .description('Collect gas from the gas service')
        .addOption(new Option('--capId <capId>', 'ID of the GasCollectorCap to borrow'))
        .action((receiver, amount, options) => mainProcessor(collectGas, [receiver, amount], options));

    const refundGasCmd = program
        .command('refund-gas <messageId> <receiver> <amount>')
        .description('Refund gas from the gas service')
        .addOption(new Option('--capId <capId>', 'ID of the GasCollectorCap to borrow'))
        .action((messageId, receiver, amount, options) => mainProcessor(refundGas, [messageId, receiver, amount], options));

    addBaseOptions(program);
    addBaseOptions(callContractCmd);
    addBaseOptions(collectGasCmd);
    addBaseOptions(refundGasCmd);

    program.parse();
}
