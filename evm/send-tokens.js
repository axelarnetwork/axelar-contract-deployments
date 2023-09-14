'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider, utils : {parseEther}, } = ethers;
const { Command, Option } = require('commander');
const chalk = require('chalk');
const { printInfo, printWalletInfo, isValidPrivateKey, isNumber, isAddressArray, getCurrentTimeInSeconds } = require('./utils');
const { getLedgerWallet, sendTx, ledgerSign, storeTransactionsData } = require('./offline-sign-utils.js');
const readlineSync = require('readline-sync');

async function sendTokens(chain, options) {
    const {privateKey, amount, recipients, offline, env} = options;
    env = (env === "local") ? "testnet" : env;
    let wallet;

    const provider = getDefaultProvider(chain.rpc);
    recipients = options.recipients.split(',').map((str) => str.trim());
    amount = parseEther(options.amount);

    if (privateKey === "ledger") {
        wallet = getLedgerWallet(provider); // Need to think whether we will take path for ledger wallet from user or somewhere else like config/ or use default one
      } else {
        if(!isValidPrivateKey(privateKey)) {
            throw new Error("Private key is missing/ not provided correctly in the user info");
        }

        wallet = new Wallet(privateKey, provider);
      }
    console.log("Wallet address 1", await wallet.getAddress());

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

    for (const recipient of recipients) {
        printInfo('Recipient', recipient);
        const nonce = parseInt(getCurrentTimeInSeconds());

        if(privateKey === "ledger") {
            if(offline === "true") {
                const tx = await ledgerSign(offline, 50000, 100, nonce, env, chain, wallet, recipient, amount);
                const msg = `Transaction created at ${nonce}. This transaction will send ${amount} of native tokens to ${recipient} on chain ${chain.name} with chainId ${chain.chainId}`;
                await storeTransactionsData(undefined, undefined, msg, tx);

            }
            else {
                const signedTx = await ledgerSign(offline, 50000, 10000000000, nonce, env, chain, wallet, recipient, amount);
                console.log("Sending signed tx through provider");
                const tx = await sendTx(signedTx, provider);
                printInfo('Transaction hash', tx.hash);
            }
            }
    else {
            const tx = await wallet.sendTransaction({
                to: recipient,
                value: amount,
            });
    
            printInfo('Transaction hash', tx.hash);
    
            await tx.wait();
        }
    }
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
    program.addOption(new Option('-o, --offline <offline>', 'If this option is set as true, then ').choices(["true", "false"]).makeOptionMandatory(false));
    program.addOption(new Option('-y, --yes', 'skip prompts'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokens };
}
