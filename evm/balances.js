'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const { getDefaultProvider } = ethers;
const { Command, Option } = require('commander');
const { mainProcessor, printWalletInfo, printInfo, addEnvironmentOptions } = require('./utils');
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

    addEnvironmentOptions(program);

    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
