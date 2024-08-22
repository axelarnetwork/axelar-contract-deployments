const { Command, Option } = require('commander');
const { getWallet } = require('./utils');
const { loadConfig } = require('../evm/utils');
const { CHAIN_ENVIRONMENTS } = require('../common');
require('./cli-utils');

async function processCommand(options, _, chain) {
    await getWallet(chain, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('balances').description('Wallet balance');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(CHAIN_ENVIRONMENTS)
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );

    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));

    program.addOption(new Option('-v, --verbose', 'verbose output').default(false));

    program.action((options) => {
        const config = loadConfig(options.env);
        processCommand(options, config, config.stellar);
    });

    program.parse();
}
