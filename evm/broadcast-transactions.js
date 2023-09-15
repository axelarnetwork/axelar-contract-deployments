'use strict';

const {
    providers: { JsonRpcProvider },
} = require('ethers');
const { Command, Option } = require('commander');
const { printError, printInfo } = require('./utils');
const {
    sendTx,
    updateTxNonceAndStatus,
    getLatestNonceFromData,
    getNonceFromProvider,
    getFilePath,
    getAllSignersData,
} = require('./offline-sign-utils');
const fs = require('fs');
const chalk = require('chalk');

async function processTransactions(directoryPath, fileName, provider, signerAddress) {
    try {
        const signersData = await getAllSignersData(directoryPath, fileName);

        if (!signersData[signerAddress]) {
            throw new Error(`Signer data for address ${signerAddress} not found in the file ${fileName}`);
        }

        let signerData = signersData[signerAddress];
        const nonceFromData = await getLatestNonceFromData(signerData);
        const nonce = parseInt(await getNonceFromProvider(provider, signerAddress));

        if (nonce > nonceFromData) {
            signerData = updateTxNonceAndStatus(signerData, nonce);
        }

        for (const transaction of signerData) {
            if (transaction.status === 'PENDING') {
                try {
                    // Send the signed transaction
                    const response = await sendTx(transaction.signedTx, provider);

                    // Update the transaction status and store transaction hash
                    transaction.status = 'SUCCESS';
                    transaction.transactionHash = response.transactionHash;
                } catch (error) {
                    // Update the transaction status and store error message
                    transaction.status = 'FAILED';
                    transaction.error = error.message;
                }
            }
        }

        // Write back the updated JSON object to the file
        signersData[signerAddress] = signerData;
        const filePath = getFilePath(directoryPath, fileName);
        fs.writeFileSync(filePath, JSON.stringify(signersData, null, 2));

        printInfo('Transactions processed successfully.');
    } catch (error) {
        printError('Error processing transactions:', error.message);
    }
}

async function main(options) {
    const { directoryPath, fileName, rpcUrl, signerAddress } = options;
    const provider = new JsonRpcProvider(rpcUrl);
    const network = await provider.getNetwork();

    if (!options.yes) {
        const anwser = fs.readlineSync.question(
            `Proceed with the broadcasting of all pending signed transactions for address ${chalk.green(
                signerAddress,
            )} on network ${chalk.green(network.name)} with chainId ${chalk.green(network.chainId)} ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    await processTransactions(directoryPath, fileName, provider, signerAddress);
}

const program = new Command();

program.name('broadcast-transactions').description('Broadcast all the pending signed transactions of the signer');

program.addOption(
    new Option('-d, --directoryPath <directoryPath>', 'The folder where all the signed tx files are stored').makeOptionMandatory(true),
);
program.addOption(new Option('-f, --fileName <fileName>', 'The fileName where the signed tx are stored').makeOptionMandatory(true));
program.addOption(
    new Option('-r, --rpcUrl <rpcUrl>', 'The rpc url for creating a provider to broadcast the transactions').makeOptionMandatory(true),
);
program.addOption(new Option('-s, --signerAddress <signerAddress>', 'private key').makeOptionMandatory(true));
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});

program.parse();
