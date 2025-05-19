const { Command } = require('commander');
const { loadConfig, saveConfig, getChainConfig, printInfo } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    getWallet,
} = require('./utils');
const {
    makeContractCall,
    PostConditionMode,
    AnchorMode,
    broadcastTransaction,
    Cl,
} = require('@stacks/transactions');
const { parseSuiUnitAmount } = require('../sui/utils');

async function collectFees(stacksAddress, privateKey, networkType, chain, args, options) {
    const contracts = chain.contracts;
    if (!contracts.GasService?.address) {
        throw new Error(`Contract GasService not yet deployed`);
    }
    if (!contracts.GasImpl?.address) {
        throw new Error(`Contract GasImpl not yet deployed`);
    }

    printInfo('Collecting gas fees');

    const unitAmount = options.amount;
    const receiver = options.receiver || stacksAddress;

    const gasServiceAddress = contracts.GasService.address.split('.');
    const registerTransaction = await makeContractCall({
        contractAddress: gasServiceAddress[0],
        contractName: gasServiceAddress[1],
        functionName: 'collect-fees',
        functionArgs: [
            Cl.address(contracts.GasImpl.address),
            Cl.address(receiver),
            Cl.uint(unitAmount),
        ],
        senderKey: privateKey,
        network: networkType,
        postConditionMode: PostConditionMode.Allow,
        anchorMode: AnchorMode.Any,
        fee: 10_000,
    });
    const result = await broadcastTransaction({
        transaction: registerTransaction,
        network: networkType,
    });

    printInfo(`Finished collecting fees`, result.txid);
}

async function processCommand(command, chain, args, options) {
    const { privateKey, stacksAddress, networkType } = await getWallet(chain, options);

    await command(stacksAddress, privateKey, networkType, chain, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(command, chain, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('Gas Service Commands').description('Stacks GasService scripts');

    const collectFeesCmd = new Command()
        .name('collect-fees')
        .description('Collect fees from gas service contract')
        .command('collect-fees')
        .option('--receiver <receiver>', 'Receiver address. Default is the sender address.')
        .requiredOption('--amount <amount>', 'Amount to collect gas', parseSuiUnitAmount)
        .action((options) => {
            mainProcessor(collectFees, options, [], processCommand);
        });

    program.addCommand(collectFeesCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
