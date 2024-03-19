'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { parseEther, parseUnits },
} = ethers;
const { printInfo, printError, printWalletInfo, isAddressArray, mainProcessor, isValidDecimal, prompt, getGasOptions } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { storeSignedTx, getWallet, signTransaction } = require('./sign-utils.js');

async function processCommand(_, chain, options) {
    const { privateKey, offline, env } = options;
    let { amount: amountStr, recipients, nonceOffset } = options;

    const chainName = chain.name.toLowerCase();
    const provider = getDefaultProvider(chain.rpc);

    recipients = options.recipients.split(',').map((str) => str.trim());

    if (!isAddressArray(recipients)) {
        throw new Error('Invalid recipient addresses');
    }

    if (!amountStr && options.gasUsage) {
        const gasPrice = parseUnits((await provider.getGasPrice()).toString(), 'wei');
        const gas = gasPrice * parseInt(options.gasUsage);
        amountStr = (gas / 1e18).toString();
    }

    if (!isValidDecimal(amountStr)) {
        throw new Error(`Invalid amount ${amountStr}`);
    }

    const amount = parseEther(amountStr);

    const wallet = await getWallet(privateKey, provider, options);

    const { address, balance } = await printWalletInfo(wallet, options);

    if (!offline) {
        if (balance.lte(amount)) {
            printError(`Wallet balance ${balance} has insufficient funds for ${amount}.`);
            return;
        }
    }

    const gasOptions = await getGasOptions(chain, options);

    if (
        prompt(
            `Proceed with the transfer of ${chalk.green(amountStr)} ${chalk.green(chain.tokenSymbol)} to ${recipients} on ${chain.name}?`,
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
            ...gasOptions,
        };

        if (!offline && chain.name.toLowerCase() === 'binance') {
            tx.gasPrice = (await provider.getGasPrice()) * 1.2;
        }

        const { baseTx, signedTx } = await signTransaction(wallet, chain, tx, options);

        if (offline) {
            const filePath = `./tx/signed-tx-${env}-send-tokens-${chainName}-address-${address}-nonce-${baseTx.nonce}.json`;
            printInfo(`Storing signed Tx offline in file ${filePath}`);

            // Storing the fields in the data that will be stored in file
            const data = {
                msg: `This transaction will send ${amount} of native tokens from ${address} to ${recipient} on chain ${chain.name}`,
                unsignedTx: baseTx,
                signedTx,
                status: 'PENDING',
            };

            storeSignedTx(filePath, data);

            nonceOffset = (parseInt(nonceOffset) || 0) + 1;
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('send-tokens').description('Send native tokens to an address.');

    addBaseOptions(program);

    program.addOption(new Option('-r, --recipients <recipients>', 'comma-separated recipients of tokens').makeOptionMandatory(true));
    program.addOption(new Option('-a, --amount <amount>', 'amount to transfer (in terms of ETH)'));
    program.addOption(new Option('--gasUsage <gasUsage>', 'amount to transfer based on gas usage and gas price').default('50000000'));
    program.addOption(new Option('--offline', 'Run in offline mode'));
    program.addOption(new Option('--nonceOffset <nonceOffset>', 'The value to add in local nonce if it deviates from actual wallet nonce'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokens: processCommand };
}
