const { Command, Option } = require('commander');
const { mainProcessor } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');

async function trustSet(_config, wallet, client, _chain, options, args) {
    await client.sendTrustSet(
        wallet,
        {
            account: options.account,
            value: options.limit,
            currency: args.currency,
            issuer: args.issuer,
        },
        options,
    );
}

if (require.main === module) {
    const program = new Command();

    program
        .name('trust-set')
        .description('Establish a trust line with the issuer of a given token.')
        .arguments('<tokenCurrency> <tokenIssuer>')
        .addOption(new Option('-m, --multisign', 'active wallet is a signer of the target XRPL multisig account').default(false))
        .addOption(new Option('--account <account>', 'XRPL account from which to create a trust line (default: active wallet)'))
        .addOption(new Option('--limit <limit>', 'trust line limit').default('1000000000'))
        .action((currency, issuer, options) => {
            mainProcessor(trustSet, options, { currency, issuer });
        });

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.parse();
}
