'use strict';

require('dotenv').config();

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { parseEther },
} = ethers;

const { printInfo, printWalletInfo, isAddressArray, mainProcessor, isValidDecimal, prompt } = require('./utils');
const { storeSignedTx, getWallet, signTransaction } = require('./sign-utils.js');

async function processCommand(_, chain, options) {
    const { privateKey, offline, env } = options;
    let { amount, recipients } = options;

    const chainName = chain.name.toLowerCase();
    const provider = getDefaultProvider(chain.rpc);

    recipients = options.recipients.split(',').map((str) => str.trim());

    if (!isAddressArray(recipients)) {
        throw new Error('Invalid recipient addresses');
    }

    if (!isValidDecimal(amount)) {
        throw new Error('Invalid amount');
    }

    amount = parseEther(amount);

    const wallet = await getWallet(privateKey, provider, options);

    const { address, balance } = await printWalletInfo(wallet, options);

    if (!offline) {
        if (balance.lte(amount)) {
            throw new Error(`Wallet has insufficient funds.`);
        }
    }

    if (
        prompt(
            `Proceed with the transfer of ${chalk.green(options.amount)} ${chalk.green(chain.tokenSymbol)} to ${recipients} on ${
                chain.name
            }?`,
            options.yes,
        )
    ) {
        printInfo('Operation Cancelled');
        return;
    }

    for (const recipient of recipients) {
        printInfo('Recipient', recipient);
        const tx = {
            to: recipient,
            value: amount,
        };

        const { baseTx, signedTx } = await signTransaction(wallet, chain, tx, options);

        if (offline) {
            const filePath = `./tx/signed-tx-${env}-${chainName}-send-tokens-address-${address}-nonce-${baseTx.nonce}.json`;
            printInfo(`Storing signed Tx offline in file ${filePath}`);

            // Storing the fields in the data that will be stored in file
            const data = {
                msg: `This transaction will send ${amount} of native tokens from ${address} to ${recipient} on chain ${chain.name}`,
                unsignedTx: baseTx,
                signedTx,
                status: 'PENDING',
            };

            storeSignedTx(filePath, data);

            options.nonceOffset = (options.nonceOffset || 0) + 1;
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
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
    program.addOption(new Option('--ledgerPath <ledgerPath>', 'The path to identify the account in ledger'));
    program.addOption(new Option('--nonceOffset <nonceOffset>', 'The value to add in local nonce if it deviates from actual wallet nonce'));
    program.addOption(new Option('-y, --yes', 'skip prompts'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokens: processCommand };
}
