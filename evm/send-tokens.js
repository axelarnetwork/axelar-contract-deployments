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

const { printInfo, printWalletInfo, isValidNumber, loadConfig } = require('./utils');
const {
    storeSignedTx,
    getWallet,
    ledgerSign,
    getLocalNonce,
} = require('./offline-sign-utils.js');

async function sendTokens(chain, options) {
    const { privateKey, offline, nonceOffset, env } = options;
    let { amount, recipients } = options;
    const provider = getDefaultProvider(chain.rpc);
    recipients = options.recipients.split(',').map((str) => str.trim());
    amount = parseEther(amount);

    const wallet = await getWallet(privateKey, provider, options);
    const signerAddress = await wallet.getAddress();
    let nonce;

    if (!offline) {
        const balance = await printWalletInfo(wallet);

        if (balance.lte(amount)) {
            throw new Error(`Wallet has insufficient funds.`);
        }
    }

    if (!options.yes) {
        const anwser = readlineSync.question(
            `Proceed with the transfer of ${chalk.green(options.amount)} ${chalk.green(chain.tokenSymbol)} to ${recipients} on ${
                chain.name
            }? ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    const gasOptions = chain.staticGasOptions || {};

    if (offline) {
        nonce = getLocalNonce(chain, signerAddress);
        if (nonceOffset) {
            if (!isValidNumber(nonceOffset)) {
                throw new Error('Provided nonce offset is not a valid number');
            }

            nonce += parseInt(nonceOffset);
        }

    }

    for (const recipient of recipients) {
        printInfo('Recipient', recipient);
        const tx = {
            to: recipient,
            value: amount,
        };

        if (offline) {
            const filePath = `./tx/signed-tx-${env}-${chain.name.toLowerCase()}-send-tokens-address-${signerAddress}-nonce-${nonce}.txt`;
            printInfo(`Storing signed Tx offline in file ${filePath}`);
            const data = {};
            tx.nonce = nonce;
            tx.chainId = chain.chainId;
            printInfo('Waiting for user to approve transaction through ledger wallet');
            const { baseTx, signedTx } = await ledgerSign(wallet, chain, tx, gasOptions);
            // Storing the fields in the data that will be stored in file
            data.msg = `This transaction will send ${amount} of native tokens to ${recipient} on chain ${chain.name} with chainId ${chain.chainId}`;
            data.unsignedTx = baseTx;
            data.signedTx = signedTx;
            data.status = 'PENDING';
            storeSignedTx(filePath, data);
        } else {
            const response = await wallet.sendTransaction(tx);
            await response.wait();
            printInfo('Transaction hash', response.transactionHash);
        }

        ++nonce;
    }
}

async function main(options) {

    const config = loadConfig(options.env);

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
    program.addOption(new Option('--offline', 'Run in offline mode'));
    program.addOption(new Option('--ledgerPath <ledgerPath>', 'The path to identify the account in ledger').makeOptionMandatory(false));
    program.addOption(
        new Option(
            '--nonceOffset <nonceOffset>',
            'The value to add in local nonce if it deviates from actual wallet nonce',
        ).makeOptionMandatory(false),
    );
    program.addOption(new Option('-y, --yes', 'skip prompts'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokens };
}
