const { Command, Option } = require('commander');
const { mainProcessor, hex } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');
const { printInfo } = require('../common');

async function accountSet(_config, wallet, client, _chain, options) {
    printInfo('Updating account properties');
    await client.sendAccountSet(wallet, {
        account: options.account,
        transferRate: options.transferRate ? Number(options.transferRate) : undefined,
        tickSize: options.tickSize ? Number(options.tickSize) : undefined,
        domain: options.domain ? hex(options.domain) : undefined,
        flag: options.flag ? Number(options.flag) : undefined,
    }, options);

    printInfo('Successfully updated account properties');
}

if (require.main === module) {
    const program = new Command();

    program
        .name('account-set')
        .description('Configure an XRPL account\'s properties')
        .addOption(new Option('-m, --multisign', 'active wallet is a signer of the XRPL multisig account being configured').default(false))
        .addOption(new Option('--account <account>', 'XRPL account to configure (default: active wallet)'))
        .addOption(new Option('--transferRate <transferRate>', 'account transfer rate'))
        .addOption(new Option('--tickSize <tickSize>', 'account tick size'))
        .addOption(new Option('--domain <domain>', 'account domain'))
        .addOption(new Option('--flag <flag>', 'account flag'));

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.action((options) => {
        mainProcessor(accountSet, options);
    });

    program.parse();
}
