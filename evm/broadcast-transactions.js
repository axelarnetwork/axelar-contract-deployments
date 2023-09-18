'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    providers: { JsonRpcProvider },
} = ethers;
const readlineSync = require('readline-sync');

const { printError, printInfo, printObj } = require('./utils');
const {
    sendTx,
    getAllSignersData,
    updateSignersData,
} = require('./offline-sign-utils');

async function processTransactions(filePath, provider) {
    try {
        const signersData = await getAllSignersData(filePath);

        for (const [signerAddress, transactions] of Object.entries(signersData)) {
            for (const transaction of transactions) {
                if (transaction.status === 'PENDING') {
                    printInfo('Broadcasting transaction: ');
                    printObj(transaction.unsignedTx);

                        // Send the signed transaction
                        const {success, response} = await sendTx(transaction.signedTx, provider);

                        if(success) {
                            // Update the transaction status and store transaction hash
                            transaction.status = 'SUCCESS';
                            transaction.transactionHash = response.transactionHash;
                            printInfo(`Transaction executed successfully ${response.transactionHash}`);
                        }
                        else {
                            // Update the transaction status and store error message
                            transaction.status = 'FAILED';
                            printError("Error broadcasting tx: ", transaction.signedTx);
                        }   
                }
            }
            // Write back the updated JSON object to the file
            signersData[signerAddress] = transactions;
          }
          updateSignersData(filePath, signersData);

    } catch (error) {
        printError('Error processing transactions:', error.message);
    }
}

async function main(options) {
    // TODO: Enable multiple scripts to use offlineSigning
    const { filePath, rpcUrl } = options;
    const provider = new JsonRpcProvider(rpcUrl);
    const network = await provider.getNetwork();

    if (!options.yes) {
        const anwser = readlineSync.question(
            `Proceed with the broadcasting of all pending signed transactions for file ${chalk.green(options.filePath)} on network ${chalk.green(network.name)} with chainId ${chalk.green(network.chainId)} ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    await processTransactions(filePath, provider);
}

const program = new Command();

program.name('broadcast-transactions').description('Broadcast all the pending signed transactions of the signer');

program.addOption(new Option('--filePath <filePath>', 'The file where the signed tx are stored').makeOptionMandatory(true));
program.addOption(
    new Option('-r, --rpcUrl <rpcUrl>', 'The rpc url for creating a provider to broadcast the transactions').makeOptionMandatory(true),
);
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});

program.parse();
