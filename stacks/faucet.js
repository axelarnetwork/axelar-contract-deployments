'use strict';

const { saveConfig, loadConfig, printInfo, printWarn, getChainConfig } = require('../common/utils');
const { getWallet, addBaseOptions } = require('./utils');
const { Command } = require('commander');

async function processCommand(config, chain, options) {
    const { stacksAddress } = await getWallet(chain, options);

    printInfo('Calling faucet for Stacks Address', stacksAddress);

    try {
        const response = await fetch(`https://api.testnet.hiro.so/extended/v1/faucets/stx?address=${stacksAddress}`, {
            method: 'POST',
        });

        if (!response.ok) {
            printWarn('Funds could not be requested...');
            return;
        }

        const data = await response.json();

        if (!data.success) {
            printWarn('Funds could not be requested...');
        } else {
            printInfo('Funds requested', stacksAddress);
            printInfo('Tx id', data.txId);
        }
    } catch (e) {
        printWarn('Funds could not be requested from Stacks faucet...');
    }
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(config, chain, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('faucet')
        .description('Query the faucet for funds.')
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program);

    program.parse();
}
