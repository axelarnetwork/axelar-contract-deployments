const { saveConfig, printInfo } = require('../evm/utils');
const { Command, Option } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

async function callContractWithCapability(keypair, client, config, chain, args, options) {
    if (!chain.contracts.operators) {
        throw new Error('Operators package not found.');
    }

    const contractConfig = chain.contracts.operators;
    const packageId = contractConfig.address;
    const [contractId, functionName, ...functionArgs] = args;

    const tx = new TransactionBlock();

    const [operatorCap] = tx.moveCall({
        target: `${packageId}::operators::borrow_cap`,
        arguments: [tx.object(contractConfig.objects.operators), tx.object(contractConfig.objects.operator_cap), tx.pure(options.capId)],
    });

    tx.moveCall({
        target: `${contractId}::${functionName}`,
        arguments: [operatorCap, ...functionArgs.map((arg) => tx.pure(arg))],
    });

    await broadcast(client, keypair, tx);

    printInfo('Contract called with borrowed capability');
}

async function mainProcessor(processor, args, options) {
    const config = loadSuiConfig(options.env);

    const [keypair, client] = getWallet(config.sui, options);
    await printWalletInfo(keypair, client, config.sui, options);

    await processor(keypair, client, config, config.sui, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('operators').description('Operators contract operations.');

    const callContractCmd = program
        .command('call-contract <contractId> <functionName> [functionArgs...]')
        .description('Call a contract with a borrowed capability')
        .addOption(new Option('--capId <capId>', 'ID of the capability to borrow'))
        .action((contractId, functionName, functionArgs, options) =>
            mainProcessor(callContractWithCapability, [contractId, functionName, ...functionArgs], options),
        );

    addBaseOptions(program);
    addBaseOptions(callContractCmd);

    program.parse();
}
