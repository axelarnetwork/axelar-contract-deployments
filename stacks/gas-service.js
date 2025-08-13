const { Command } = require('commander');
const { loadConfig, saveConfig, getChainConfig, printInfo } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet } = require('./utils');
const { Cl } = require('@stacks/transactions');
const { sendContractCallTransaction } = require('./utils/sign-utils');

async function collectFees(wallet, chain, args, options) {
    const contracts = chain.contracts;
    if (!contracts.AxelarGasService?.address) {
        throw new Error(`Contract GasService not yet deployed`);
    }
    if (!contracts.GasImpl?.address) {
        throw new Error(`Contract GasImpl not yet deployed`);
    }

    printInfo('Collecting gas fees');

    const unitAmount = options.amount;
    const receiver = options.receiver || wallet.stacksAddress;

    const result = await sendContractCallTransaction(
        contracts.AxelarGasService.address,
        'collect-fees',
        [Cl.address(contracts.GasImpl.address), Cl.address(receiver), Cl.uint(unitAmount)],
        wallet,
    );

    printInfo(`Finished collecting fees`, result.txid);
}

async function processCommand(command, chain, args, options) {
    const wallet = await getWallet(chain, options);

    await command(wallet, chain, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
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
        .requiredOption('--amount <amount>', 'Amount to collect gas')
        .action((options) => {
            mainProcessor(collectFees, options, [], processCommand);
        });

    program.addCommand(collectFeesCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
