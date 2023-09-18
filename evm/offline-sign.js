'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const fs = require('fs');
const { ethers } = require('hardhat');
const { getDefaultProvider } = ethers;

const { printError, printInfo, printObj } = require('./utils');
const {
    getWallet,
    getTransactions,
    getAllSignersData,
    updateSignersData,
} = require('./offline-sign-utils');

async function processTransactions(wallet, signerAddress, filePath) {
    try {
        let signersData = await getAllSignersData(filePath);
        let transactions = await getTransactions(filePath, signerAddress);

        for(const transaction of transactions) {
            if(transaction.status === "NOT_SIGNED") {
                const signedTx = await wallet.signTransaction(transaction.unsignedTx);
                transaction.status = "PENDING";
                transaction.signedTx = signedTx;
            }
        }

        signersData[signerAddress] = transactions;
        await updateSignersData(filePath, signersData);
        printInfo(`Transactions signed succesfully and stored in file ${filePath}`);

    } catch(error) {
        printError(`Transactions signing failed with error: ${error.message}`);
        printObj(error);
    }
}


async function main(options) {
    const { filePath, ledgerPath, rpcUrl } = options;
    const provider = getDefaultProvider(rpcUrl);
    const network = await provider.getNetwork();
    const wallet = getWallet("ledger", provider, ledgerPath);
    const signerAddress = await wallet.getAddress();

    if (!options.yes) {
        const anwser = fs.readlineSync.question(
            `Proceed with the signing of all unSigned transactions for address ${chalk.green(
                signerAddress,
            )} on network ${chalk.green(network.name)} with chainId ${chalk.green(network.chainId)} ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    await processTransactions(wallet, signerAddress, filePath);
}

const program = new Command();

program.name('Offline-Signing').description('Offline sign all the unsigned transactions in the file');

program.addOption(new Option('-f, --filePath <filePath>', 'The filePath where the signed tx will be stored').makeOptionMandatory(true));
program.addOption(
    new Option('-r, --rpcUrl <rpcUrl>', 'The rpc url for creating a provider to sign the transactions').makeOptionMandatory(true),
);
program.addOption(new Option('--ledgerPath <ledgerPath>', 'The path to identify the account in ledger').makeOptionMandatory(false));
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});

program.parse();