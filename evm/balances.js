'use strict';

const { ethers } = require('hardhat');
const { getDefaultProvider } = ethers;
const { Command } = require('commander');
const { mainProcessor, printWalletInfo } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');

async function processCommand(_axelar, chain, _chains, options) {
    const provider = getDefaultProvider(chain.rpc);

    const wallet = await getWallet(options.privateKey, provider);
    await printWalletInfo(wallet, options, chain);
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
