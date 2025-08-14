'use strict';

const { Command } = require('commander');
const { addBaseOptions, createStacksWallet } = require('./utils');
const { saveConfig, loadConfig, printInfo, getChainConfig } = require('../common/utils');

async function processCommand(config, chain) {
    const { mnemonic, stacksAddress } = await createStacksWallet(chain);

    chain.initialContractsDeployer = stacksAddress;

    printInfo('Wallet generated');
    printInfo('Mnemonic', mnemonic);
    printInfo('Address', stacksAddress);
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(config, chain, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('generate-wallet').description('Generate wallet.');

    addBaseOptions(program, { ignorePrivateKey: true });

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
