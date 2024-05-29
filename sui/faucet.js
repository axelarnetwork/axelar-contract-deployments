'use strict';

const { addBaseOptions } = require('./cli-utils');
const { requestSuiFromFaucetV0, getFaucetHost } = require('@mysten/sui.js/faucet');
const { getWallet } = require('./sign-utils');
const { Command } = require('commander');
const { saveConfig, loadConfig, printInfo } = require('../evm/utils');

async function processCommand(config, chain, options) {
    const [keypair] = await getWallet(chain, options);

    await requestSuiFromFaucetV0({
        host: getFaucetHost(chain.networkType),
        recipient: keypair.toSuiAddress(),
    });

    printInfo('Funds requested');
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('faucet').description('Query the faucet for funds.');

    addBaseOptions(program);

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
