'use strict';

const xrpl = require('xrpl');
const chalk = require('chalk');
const {
    loadConfig,
    saveConfig,
    printInfo,
    printError,
} = require('../common');

const hex = (str) => Buffer.from(str).toString("hex");

async function getAccountInfo(client, address) {
    const accountInfo = await client.request({
        command: 'account_info',
        account: address,
        ledger_index: 'validated',
    });
    return accountInfo.result.account_data;
}

async function getFee(client) {
    const fee = await client.request({ command: "fee" });
    return fee.result.drops.open_ledger_fee;
}

async function sendTransaction(client, signer, tx) {
    const prepared = await client.autofill(tx);
    const signed = signer.sign(prepared);
    const receipt = await client.submitAndWait(signed.tx_blob);
    if (receipt.result.meta.TransactionResult !== 'tesSUCCESS') {
        printError('Transaction failed', txRes.result);
        throw new Error(`Transaction failed: ${receipt.result.meta.TransactionResult}`);
    }

    return receipt;
}

async function getWallet(chain, options) {
    const client = new xrpl.Client(chain.rpc);
    await client.connect();

    const wallet = xrpl.Wallet.fromSeed(options.seed);
    const address = wallet.address;

    try {
        const accountInfo = await getAccountInfo(client, address);
        const balance = accountInfo.Balance;
        const sequence = accountInfo.Sequence;

        printInfo('Wallet address', address);
        printInfo('Wallet balance', `${balance / 1e6} ${chain.tokenSymbol || ''}`);
        printInfo('Wallet sequence', sequence);
    } catch (error) {
        if (error.data.error !== 'actNotFound') {
            printError('Failed to get account info for wallet', address);
            throw error;
        }
    } finally {
        await client.disconnect();
    }

    return wallet;
}

const mainProcessor = async (options, processCommand, save = true, catchErr = false) => {
    if (!options.env) {
        throw new Error('Environment was not provided');
    }

    printInfo('Environment', options.env);

    const config = loadConfig(options.env);

    if (options.chainName === undefined) {
        throw new Error('Chain name was not provided');
    }

    const chainName = options.chainName.toLowerCase();

    if (config.chains[chainName] === undefined) {
        throw new Error(`Chain ${chainName} is not defined in the info file`);
    }

    if (config.chains[chainName].chainType !== 'xrpl') {
        throw new Error(`Cannot run script for a non XRPL chain: ${chainName}`);
    }

    const chain = config.chains[chainName];

    console.log('');
    printInfo('Chain', chain.name, chalk.cyan);

    try {
        await processCommand(config, chain, options);
    } catch (error) {
        printError(`Failed with error on ${chain.name}`, error.message);

        if (!catchErr && !options.ignoreError) {
            throw error;
        }
    }

    if (save) {
        saveConfig(config, options.env);
    }
};

module.exports = {
    ...require('../common/utils'),
    getWallet,
    getAccountInfo,
    getFee,
    mainProcessor,
    sendTransaction,
    hex,
};
