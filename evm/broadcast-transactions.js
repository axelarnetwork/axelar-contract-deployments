'use strict';

const {
    BigNumber,
    utils: { isAddress, serializeTransaction },
    providers: { JsonRpcProvider},
    Wallet,
} = require('ethers');
const { LedgerSigner } = require('@ethersproject/hardware-wallets');
const { printObj, isNumber, printError } = require('./utils');
const { sendTx, getNonce, updateTxNonceAndStatus, getLatestNonceFromData, getSignerData } = require('./offline-sign-utils');


async function processTransactions(dirPath, fileName, provider, signerAddress) {
    try {
        const signerData = getSignerData(dirPath, fileName, signerAddress);
        const nonceFromData = getLatestNonceFromData(signerData);
        const nonce = parseInt(await getNonceFromProvider(provider, signerAddress));
        if(nonce > nonceFromData) {
           signerData = updateTxNonceAndStatus(signerData, nonce);
        }

        for (const transaction of signerData) {
            if (transaction.status === 'PENDING') {
                try {
                    // Send the signed transaction
                    const response = await sendTx(transaction.signedTransaction, provider);

                    // Update the transaction status and store transaction hash
                    transaction.status = "SUCCESS";
                    transaction.transactionHash = response.transactionHash;
                } catch (error) {
                    // Update the transaction status and store error message
                    transaction.status = "FAILED";
                    transaction.error = error.message;
                }
            }
        }

        // Write back the updated JSON object to the file
        fs.writeFileSync(filePath, JSON.stringify(jsonData, null, 2));

        console.log('Transactions processed successfully.');
    } catch (error) {
        console.error('Error processing transactions:', error);
    }
}

async function main(options) {
    const {directoryPath, fileName, rpcUrl, signerAddress, yes} = options;
    const provider = new JsonRpcProvider(rpcUrl);
    const network = await provider.getNetwork();

    if (!options.yes) {
        const anwser = readlineSync.question(
            `Proceed with the broadcasting of all pending signed transactions for address ${chalk.green(signerAddress)} on network ${chalk.green(network.name)} with chainId ${chalk.green(network.chainId)} ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    await processTransactions(directoryPath, fileName, provider, signerAddress);
}

const program = new Command();

program.name('broadcast-transactions').description('Broadcast all the pending signed transactions of the signer');

program.addOption(new Option('-d, --directoryPath <directoryPath>', 'The folder where all the signed tx files are stored').makeOptionMandatory(true));
program.addOption(new Option('-f, --fileName <fileName>', 'The fileName where the signed tx are stored').makeOptionMandatory(true));
program.addOption(new Option('-r, --rpcUrl <rpcUrl>', 'The rpc url for creating a provider to broadcast the transactions').makeOptionMandatory(true));
program.addOption(new Option('-s, --signerAddress <signerAddress>', 'private key').makeOptionMandatory(true));
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});

program.parse();