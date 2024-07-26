'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const { getDefaultProvider } = ethers;

const { mainProcessor, printInfo, prompt } = require('./utils');
const { addBaseOptions } = require('../common');
const { getNonceFromProvider, getNonceFileData, updateNonceFileData } = require('./sign-utils');

async function processCommand(_, chain, options) {
    const { env, rpc, yes } = options;
    let { addresses } = options;
    const provider = rpc ? getDefaultProvider(rpc) : getDefaultProvider(chain.rpc);

    if (prompt(`Proceed with the nonces update on network ${chalk.green(chain.name)}`, yes)) {
        return;
    }

    const chainName = chain.name.toLowerCase();
    const nonceData = getNonceFileData();

    if (!nonceData[env]) {
        nonceData[env] = {};
    }

    if (!nonceData[env][chainName]) {
        nonceData[env][chainName] = {};
    }

    const chainNonceData = nonceData[env][chainName];

    if (addresses) {
        addresses = addresses.split(',').map((str) => str.trim());
    } else if (chainNonceData.length > 0) {
        addresses = Object.keys(chainNonceData);
    } else if (chain.contracts?.Multisig?.signers) {
        addresses = chain.contracts.Multisig.signers;
    } else {
        throw new Error('No addresses provided or found for chain');
    }

    for (const address of addresses) {
        printInfo(`Updating nonce on ${chain.name} for address`, address);
        const nonce = await getNonceFromProvider(provider, address);
        chainNonceData[address] = nonce;
    }

    updateNonceFileData(nonceData);
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('update-nonces').description('Update nonces for addresses');

    addBaseOptions(program, { ignorePrivateKey: true });

    program.addOption(new Option('-r, --rpc <rpc>', 'The rpc url for creating a provider to fetch gasOptions'));
    program.addOption(new Option('--addresses <addresses>', 'The Array of addresses for which the nonces to update').env('ADDRESSES'));

    program.action((options) => {
        main(options);
    });
    program.parse();
}
