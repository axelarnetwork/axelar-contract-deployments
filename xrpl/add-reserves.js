const { Command, Option } = require('commander');
const xrpl = require('xrpl');
const { mainProcessor, hex } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');

async function addReserves(_config, wallet, client, chain, options, _args) {
    await client.sendPayment(
        wallet,
        {
            destination: chain.contracts.AxelarGateway.address,
            amount: xrpl.xrpToDrops(options.amount),
            memos: [{ memoType: hex('type'), memoData: hex('add_reserves') }],
        },
        options,
    );
}

if (require.main === module) {
    const program = new Command();

    program
        .name('add-reserves')
        .description('Top up the XRPL multisig fee reserve with XRP.')
        .addOption(new Option('--amount <amount>', 'amount of XRP to deposit into the fee reserve').makeOptionMandatory(true))
        .action((options) => {
            mainProcessor(addReserves, options);
        });

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.parse();
}
