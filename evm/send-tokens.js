'use strict';

require('dotenv').config();

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { parseEther },
} = ethers;
const readlineSync = require('readline-sync');

const { printInfo, printWalletInfo, isValidNumber } = require('./utils');
const {
    getAllSignersData,
    updateSignersData,
    getTransactions,
    getWallet,
    ledgerSign,
    getLocalNonce,
    updateLocalNonce,
} = require('./offline-sign-utils.js');

async function sendTokens(chain, options) {
    const { privateKey, offline, ledgerPath, filePath, nonceFilePath, nonceOffset } = options;
    let { amount, recipients} = options;

    if(!nonceFilePath) {
        throw new Error("Nonce FilePath is not provided in user info");
    }

    const provider = getDefaultProvider(chain.rpc);
    recipients = options.recipients.split(',').map((str) => str.trim());
    amount = parseEther(amount);

    const { wallet, providerNonce } = await getWallet(privateKey, provider, ledgerPath);
    const signerAddress = await wallet.getAddress();
    let nonce = getLocalNonce(nonceFilePath, signerAddress);

    if(providerNonce > nonce) {
        updateLocalNonce(nonceFilePath, signerAddress, providerNonce);
        nonce = providerNonce;
    }

    const balance = await printWalletInfo(wallet);

    if (balance.lte(amount)) {
        throw new Error(`Wallet has insufficient funds.`);
    }

    if (!options.yes) {
        const anwser = readlineSync.question(
            `Proceed with the transfer of ${chalk.green(options.amount)} ${chalk.green(chain.tokenSymbol)} to ${recipients} on ${
                chain.name
            }? ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    let signersData, transactions;
    const gasOptions = chain.staticGasOptions || {};

    if (offline) {
        if(!filePath) {
            throw new Error("FilePath is not provided in user info");
        }
        if(nonceOffset) {
            if(!isValidNumber(nonceOffset)) {
                throw new Error("Provided nonce offset is not a valid number");
            }
            nonce += parseInt(nonceOffset);
        }
        printInfo(`Storing signed Txs offline in file ${filePath}`);
        signersData = await getAllSignersData(filePath);
        transactions = await getTransactions(filePath, signerAddress);
    }

    for (const recipient of recipients) {
        printInfo('Recipient', recipient);
        const tx = {
            to: recipient,
            value: amount,
        }

            if (offline) {
                const data = {};
                tx.nonce = nonce;
                tx.chainId = chain.chainId;
                const {baseTx, signedTx} = await ledgerSign(wallet, chain, tx, gasOptions);
                // Storing the fields in the data that will be stored in file
                data.msg = `This transaction will send ${amount} of native tokens to ${recipient} on chain ${chain.name} with chainId ${chain.chainId}`;
                data.unsignedTx = baseTx; 
                data.signedTx = signedTx;     
                data.status = "PENDING";          
                transactions.push(data);
            }
            else {
            const tx = await wallet.sendTransaction(tx);

            printInfo('Transaction hash', tx.hash);

            await tx.wait();
        }

        ++nonce;
    }

    if (transactions) {
        signersData[signerAddress] = transactions;
        await updateSignersData(filePath, signersData);
    }
    // Updating Nonce data for this Address
    updateLocalNonce(nonceFilePath, signerAddress, nonce);
}

async function main(options) {
    const config = require(`${__dirname}/../axelar-chains-config/info/${options.env === 'local' ? 'testnet' : options.env}.json`);

    const chains = options.chainNames.split(',').map((str) => str.trim());

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];

        await sendTokens(chain, options);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('send-tokens').description('Send native tokens to an address.');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-r, --recipients <recipients>', 'comma-separated recipients of tokens').makeOptionMandatory(true));
    program.addOption(new Option('-a, --amount <amount>', 'amount to transfer (in terms of ETH)').makeOptionMandatory(true));
    program.addOption(
        new Option('--offline', 'Run in offline mode'),
    );
    program.addOption(new Option('--ledgerPath <ledgerPath>', 'The path to identify the account in ledger').makeOptionMandatory(false));
    program.addOption(
        new Option('--filePath <filePath>', 'The filePath where the signed tx will be stored').makeOptionMandatory(false),
    );
    program.addOption(new Option('--nonceFilePath <nonceFilePath>', 'The File where nonce value to use for each address is stored').makeOptionMandatory(false));
    program.addOption(new Option('--nonceOffset <nonceOffset>', 'The value to add in local nonce if it deviates from actual wallet nonce').makeOptionMandatory(false));
    program.addOption(new Option('-y, --yes', 'skip prompts'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokens };
}
