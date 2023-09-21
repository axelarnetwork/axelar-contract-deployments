'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    providers: { getDefaultProvider },
} = ethers;
const readlineSync = require('readline-sync');

const { printError, printInfo, printObj, loadConfig } = require('./utils');
const { sendTx, getSignedTx, storeSignedTx } = require('./offline-sign-utils');

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

    try {
        if (!filePath) {
            throw new Error('FilePath is not provided in user info');
        }

        const transaction = await getSignedTx(filePath);

        if (transaction.status === 'PENDING') {
            printInfo('Broadcasting transaction: ');
            printObj(transaction.unsignedTx);

            // Send the signed transaction
            const { success, response } = await sendTx(transaction.signedTx, provider);

            if (success) {
                // Update the transaction status and store transaction hash
                transaction.status = 'SUCCESS';
                transaction.transactionHash = response.transactionHash;
                printInfo(`Transaction executed successfully ${response.transactionHash}`);
            } else {
                // Update the transaction status and store error message
                transaction.status = 'FAILED';
                printError('Error broadcasting tx: ', transaction.signedTx);
            }
        }

        storeSignedTx(filePath, transaction);
    } catch (error) {
        printError('Error processing transactions:', error.message);
    }
}

async function main(options) {
    const config = loadConfig(options.env);
    const chain = config.chains[options.chainName.toLowerCase()];

    if (chain === undefined) {
        throw new Error(`Chain ${chainName} is not defined in the info file`);
    }

    await processCommand(config, chain, options);
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
