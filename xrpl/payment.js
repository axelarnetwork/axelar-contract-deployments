const { Command, Option } = require('commander');
const { mainProcessor, parseTokenAmount } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');
const { printInfo } = require('../common');

async function payment(_config, wallet, client, _chain, options) {
    printInfo('Transferring tokens');
    await client.sendPayment(wallet, {
        account: options.from,
        amount: parseTokenAmount(options.token, options.amount), // token is either "XRP" or "<currency>.<issuer-address>"
        destination: options.to,
    }, options);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('payment')
        .description('Configure an XRPL account\'s properties')
        .addOption(new Option('-m, --multisign', 'active wallet is a signer of the sender XRPL multisig account').default(false))
        .addOption(new Option('--from <from>', 'account to send from (default: active wallet)'))
        .addOption(new Option('--to <to>', 'destination account').makeOptionMandatory(true))
        .addOption(new Option('--token <token>', 'token to send ("XRP" or "<currency>.<issuer>")').default('XRP'))
        .addOption(new Option('--amount <amount>', 'amount of tokens to send').makeOptionMandatory(true));

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.action((options) => {
        mainProcessor(payment, options);
    });

    program.parse();
}
