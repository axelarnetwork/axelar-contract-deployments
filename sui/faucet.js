'use strict';

const { requestSuiFromFaucetV0, getFaucetHost } = require('@mysten/sui/faucet');
const { saveConfig, loadConfig, printInfo } = require('../common/utils');
const { getWallet, printWalletInfo, addBaseOptions } = require('./utils');
const { Command, Option } = require('commander');

async function processCommand(config, chain, options) {
    const [keypair, client] = getWallet(chain, options);
    const recipient = options.recipient || keypair.toSuiAddress();

    await printWalletInfo(keypair, client, chain, options);

    await requestSuiFromFaucetV0({
        host: getFaucetHost(chain.networkType),
        recipient,
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

    program
        .name('faucet')
        .addOption(new Option('--recipient <recipient>', 'recipient to request funds for'))
        .description('Query the faucet for funds.')
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program);

    program.parse();
}
