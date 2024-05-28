'use strict';

const { addBaseOptions } = require('./cli-utils');
const { requestSuiFromFaucetV0, getFaucetHost } = require('@mysten/sui.js/faucet');
const { getWallet } = require('./sign-utils');
const { Command, Option } = require('commander');
const { saveConfig, loadConfig } = require('../evm/utils');

async function processCommand(_, chain, options) {
    const keypair = await getWallet(chain, options);

    await requestSuiFromFaucetV0({
        host: getFaucetHost(chain.networkType),
        recipient: keypair.toSuiAddress(),
    });
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    await processor(options, config, config.sui);
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
