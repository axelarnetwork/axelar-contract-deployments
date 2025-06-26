const { Command, Option } = require('commander');
const { mainProcessor } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');
const { printInfo } = require('../common');

async function ticketCreate(_config, wallet, client, _chain, options) {
    printInfo(`Creating ${options.ticketCount} tickets`);
    await client.sendTicketCreate(
        wallet,
        {
            account: options.account,
            ticketCount: Number(options.ticketCount),
        },
        options,
    );

    printInfo('Successfully created tickets');
}

if (require.main === module) {
    const program = new Command();

    program
        .name('ticket-create')
        .description('Create tickets for an XRPL account.')
        .addOption(new Option('-m, --multisign', 'active wallet is a signer of the target XRPL multisig account').default(false))
        .addOption(new Option('--account <account>', 'XRPL account to configure (default: active wallet)'))
        .addOption(new Option('--ticketCount <ticketCount>', 'number of tickets to create').makeOptionMandatory(true));

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.action((options) => {
        mainProcessor(ticketCreate, options);
    });

    program.parse();
}
