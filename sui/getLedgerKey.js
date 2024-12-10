'use strict';

const { saveConfig, loadConfig, getChainConfig, printInfo } = require('../common/utils');
const { getWallet, addBaseOptions } = require('./utils');
const { Command } = require('commander');

async function processCommand(config, chain, options) {
    const [keypair] = getWallet(chain, options);
    printInfo('PublicKey', (await keypair.getPublicKey()).address.toString('base64'));
    printInfo('Address', await keypair.toSuiAddress());
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(config, chain, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('getPublicKey')
        .description('Query the public key and sui address for the ledger')
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program);

    program.parse();
}
