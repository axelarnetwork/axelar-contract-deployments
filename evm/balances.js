'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const { getDefaultProvider } = ethers;
const { Command } = require('commander');
const { mainProcessor, printWalletInfo, printInfo } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');

async function processCommand(_, chain, options) {
    const provider = getDefaultProvider(chain.rpc);

    printInfo('Chain', chain.name);

    const wallet = await getWallet(options.privateKey, provider);
    await printWalletInfo(wallet, options);
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('balances').description('Display balance of the wallet on specified chains.');

    addBaseOptions(program);

    program.action((options) => {
        main(options);
    });

    program.parse();
}
