'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const { getDefaultProvider } = ethers;

const { mainProcessor, printInfo, prompt } = require('./utils');
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
    } else {
        addresses = Object.keys(chainNonceData);
    }

    for (const address of addresses) {
        printInfo('Updating nonce for address', address);
        const nonce = await getNonceFromProvider(provider, address);
        chainNonceData[address] = nonce;
    }

    updateNonceFileData(nonceData);
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

const program = new Command();

program.name('update-nonces').description('Update nonces for addresses');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(new Option('-r, --rpc <rpc>', 'The rpc url for creating a provider to fetch gasOptions'));
program.addOption(new Option('--addresses <addresses>', 'The Array of addresses for which the nonces to update').env('ADDRESSES'));
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});
program.parse();
