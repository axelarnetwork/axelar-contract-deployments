'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const fs = require('fs');
const { ethers } = require('hardhat');
const {
    getDefaultProvider
} = ethers;

const { printError, printInfo, printObj } = require('./utils');
const {
    getNonceFromProvider, getAllSignersData,
} = require('./offline-sign-utils');

function updateNonce(provider, addresses, filePath) {
    try {
        const nonceData = getAllSignersData(filePath);
        addresses = JSON.parse(addresses);

        for(const address of addresses) {
            const nonce = getNonceFromProvider(provider, address);
            nonceData[address]=  nonce;
        }

        fs.writeFileSync(filePath, JSON.stringify(nonceData, null, 2), (err) => {
            if (err) {
                printError(`Could not update Nonce in file ${filePath}`);
                printObj(err);
                return;
            }
    
        });
        printInfo(`Nonce updated succesfully and stored in file ${filePath}`);
    } catch(error) {
        printError(`Nonce updation failed with error: ${error.message}`);
        printObj(error);
    }
}


async function main(options) {
    const { filePath, rpcUrl, addresses } = options;
    const provider = getDefaultProvider(rpcUrl);
    const network = await provider.getNetwork();

    if (!options.yes) {
        const anwser = fs.readlineSync.question(
            `Proceed with the nonces update of all addresses ${chalk.green(
                addresses,
            )} on network ${chalk.green(network.name)} with chainId ${chalk.green(network.chainId)} ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    await updateNonce(provider, addresses, filePath);
}

const program = new Command();

program.name('Update-Nonces').description('Offline sign all the unsigned transactions in the file');

program.addOption(new Option('-f, --filePath <filePath>', 'The filePath where the nonce for addresses will be stored').makeOptionMandatory(true));
program.addOption(
    new Option('-r, --rpcUrl <rpcUrl>', 'The rpc url for creating a provider to sign the transactions').makeOptionMandatory(true),
);
program.addOption(new Option('-a --addresses <addresses>', 'The Array of addresses for which the nonces to update').env("ADDRESSES").makeOptionMandatory(true));
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});

program.parse();