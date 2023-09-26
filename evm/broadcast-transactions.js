'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    providers: { getDefaultProvider },
} = ethers;
const readlineSync = require('readline-sync');

const { printError, printInfo, mainProcessor } = require('./utils');
const { sendTransaction, getSignedTx, storeSignedTx } = require('./offline-sign-utils');

async function processCommand(config, chain, options) {
    const { filePath, rpc } = options;

    const provider = getDefaultProvider(rpc || chain.rpc);

    if (!options.yes) {
        const anwser = readlineSync.question(
            `Proceed with the broadcasting of all pending signed transactions for file ${chalk.green(
                options.filePath,
            )} on chain ${chalk.green(chain.name)} ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    if (!filePath) {
        throw new Error('FilePath is not provided in user info');
    }

    const transaction = await getSignedTx(filePath);

    if (transaction.status === 'PENDING') {
        printInfo('Broadcasting transaction', JSON.stringify(transaction.unsignedTx, null, 2));

        // Send the signed transaction
        const { success, response } = await sendTransaction(transaction.signedTx, provider);

        if (success) {
            // Update the transaction status and store transaction hash
            transaction.status = 'SUCCESS';
            transaction.transactionHash = response.hash;
            printInfo(`Transaction executed successfully ${response.hash}`);
        } else {
            // Update the transaction status and store error message
            transaction.status = 'FAILED';
            printError('Error broadcasting tx: ', transaction.signedTx);
        }

        storeSignedTx(filePath, transaction);
    } else {
        printInfo('Skipping broadcast, transaction status is', transaction.status);
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

const program = new Command();

program.name('broadcast-transactions').description('Broadcast all the pending signed transactions of the signer');

program.addOption(new Option('--filePath <filePath>', 'The file where the signed tx are stored').makeOptionMandatory(true));
program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-n, --chainName <chainName>', 'chain names').makeOptionMandatory(true));
program.addOption(new Option('-r, --rpc <rpc>', 'The chain rpc'));
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});

program.parse();
