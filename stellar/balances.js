const { Command } = require('commander');
const { getWallet, addBaseOptions } = require('./utils');
const { loadConfig } = require('../evm/utils');
const { getChainConfig } = require('../common');
require('./cli-utils');

async function processCommand(options, _, chain) {
    await getWallet(chain, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('balances').description('Wallet balance');

    addBaseOptions(program);

    program.action((options) => {
        const config = loadConfig(options.env);
        processCommand(options, config, getChainConfig(config, options.chainName));
    });

    program.parse();
}
