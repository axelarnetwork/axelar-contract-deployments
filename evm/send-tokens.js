'use strict';

require('dotenv').config();

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { parseEther, parseUnits },
} = ethers;
const readlineSync = require('readline-sync');

const { printInfo, printWalletInfo, isValidPrivateKey, printError, printObj } = require('./utils');
const {
    getAllSignersData,
    getNonceFromProvider,
    updateSignersData,
    getLatestNonceAndUpdateData,
    getTransactions,
    getWallet,
    getUnsignedTx,
} = require('./offline-sign-utils.js');
const { blob } = require('stream/consumers');

async function sendTokens(chain, options) {
    const { privateKey, offline, env, ledgerPath, nonceFilePath } = options;
    let { amount, recipients, filePath} = options;

    const provider = getDefaultProvider(chain.rpc);
    recipients = options.recipients.split(',').map((str) => str.trim());
    amount = parseEther(amount);

    const wallet = getWallet(privateKey, provider, ledgerPath);

    const signerAddress = await wallet.getAddress();
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

    let nonce = await getNonceFromProvider(provider, signerAddress);
    console.log("Provider nonce", nonce);
    const nonceData = getAllSignersData(nonceFilePath);
    let nonceFromFile = nonceData[signerAddress] || 0;
    console.log("Nonce file", nonceFromFile);
    nonce = (nonce >= parseInt(nonceFromFile)) ? nonce : nonceFromFile;
    console.log("Nonce after from nonce file if greatest", nonce);
    let signersData, transactions, chainId;

    if (offline) {
        filePath = filePath || env.toLowerCase() + '-' + chain.name.toLowerCase() + '-' + 'unsignedTransactions.json';
        printInfo(`Storing signed Txs offline in file ${filePath}`);
        nonce = await getLatestNonceAndUpdateData(filePath, wallet, nonce);
        console.log("Final nonce from getLatestNonce function", nonce);
        signersData = await getAllSignersData(filePath);
        transactions = await getTransactions(filePath, signerAddress);
        const network = await provider.getNetwork();
        chainId = network.chainId;
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
                tx.chainId = chainId;
                const unsignedTx = getUnsignedTx(chain, tx);
                // Storing the fields in the data that will be stored in file
                data.msg = `This transaction will send ${amount} of native tokens to ${recipient} on chain ${chain.name} with chainId ${chainId}`;
                data.unsignedTx = unsignedTx;
                data.status = 'NOT_SIGNED';
                
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
    nonceData[signerAddress] = nonce;
    await updateSignersData(nonceFilePath, nonceData);
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
    program.addOption(new Option('-y, --yes', 'skip prompts'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokens };
}
