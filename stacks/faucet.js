'use strict';

const { saveConfig, loadConfig, printInfo, printWarn, getChainConfig } = require('../common/utils');
const { getWallet, addBaseOptions } = require('./utils');
const { Command, Option } = require('commander');

async function processCommand(config, chain, options) {
    const { stacksAddress } = await getWallet(chain, options);

    printInfo('Calling faucet for Stacks Address', stacksAddress);

    const response = await fetch(`https://api.testnet.hiro.so/extended/v1/faucets/stx?address=${stacksAddress}`, {
        method: "POST",
    });
    try {
      const data = await response.json();

      if (!data.success) {
        printWarn('Funds could not be requested...');
      } else {
        printInfo('Funds requested', stacksAddress);
        printInfo('Tx id', data.txId);
      }
    } catch (e) {
      printWarn('Funds could not be requested...');
    }
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
        .name('faucet')
        .addOption(new Option('--recipient <recipient>', 'recipient to request funds for'))
        .description('Query the faucet for funds.')
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program);

    program.parse();
}
