'use strict';

const xrpl = require('xrpl');
const { decodeAccountID } = require('ripple-address-codec');
const { decode: decodeTxBlob } = require('ripple-binary-codec');
const chalk = require('chalk');
const { loadConfig, saveConfig, printInfo, printWarn, printError, prompt, getChainConfig } = require('../common');
const { prepareClient, prepareWallet } = require('../cosmwasm/utils');
const XRPLClient = require('./xrpl-client');

function hex(str) {
    return Buffer.from(str).toString('hex');
}

function getEvent(events, type) {
    if (!Array.isArray(events)) {
        throw new Error('Events list missing or not an array');
    }

    const evt = events.find((e) => e && e.type === type);
    if (!evt) {
        throw new Error(`${type} event not found`);
    }

    return evt;
}

function getEventAttr(event, attr) {
    const attrs = Array.isArray(event.attributes) ? event.attributes : [];
    const a = attrs.find((x) => x && x.key === attr);
    if (!a || typeof a.value === 'undefined') {
        throw new Error(`Attribute "${attr}" not found in ${event.type}`);
    }

    return a.value;
}

function roundUpToNearestXRP(amountInDrops) {
    return Math.ceil(amountInDrops / 1e6) * 1e6;
}

function generateWallet(options) {
    return xrpl.Wallet.generate(options.walletKeyType);
}

function getWallet(options) {
    return xrpl.Wallet.fromSeed(options.privateKey, {
        algorithm: options.walletKeyType,
    });
}

function deriveAddress(publicKey) {
    return new xrpl.Wallet(publicKey).address;
}

function decodeAccountIDToHex(accountId) {
    return hex(decodeAccountID(accountId));
}

// XRPL token is either:
// (1) "XRP"
// (2) "<currency>.<issuer-address>"
function parseTokenAmount(token, amount) {
    let parsedAmount;

    if (token === 'XRP') {
        parsedAmount = xrpl.xrpToDrops(amount);
    } else {
        const [currency, issuer] = token.split('.');
        parsedAmount = {
            currency,
            issuer,
            value: amount,
        };
    }

    return parsedAmount;
}

async function broadcastTxBlob(client, txBlob, options) {
    const tx = decodeTxBlob(txBlob);
    printInfo('Preparing to broadcast transaction', tx);

    if (prompt(`Submit ${tx.TransactionType} transaction?`, options.yes)) {
        process.exit(0);
    }

    await client.submitTx(txBlob);
}

async function printWalletInfo(client, wallet, chain) {
    const address = wallet.address;
    const { balance, sequence } = await client.accountInfo(address);
    printInfo('Wallet address', address);

    if (balance === '0') {
        printError('Wallet balance', '0');
    } else {
        printInfo('Wallet balance', `${xrpl.dropsToXrp(balance)} ${chain.tokenSymbol || ''}`);
    }

    if (sequence === '-1') {
        printWarn('Wallet is not active because it does not meet the base reserve requirement');
        return;
    }

    printInfo('Wallet sequence', sequence);

    const lines = await client.accountLines(address);

    if (lines.length === 0) {
        printInfo('Wallet IOU balances', 'No IOU balances found');
        return;
    }

    printInfo('Wallet IOU balances', lines.map((line) => `${line.balance} ${line.currency}.${line.account}`).join('  '));
}

async function mainProcessor(processor, options, args, save = true, catchErr = false) {
    if (!options.env) {
        throw new Error('Environment was not provided');
    }

    printInfo('Environment', options.env);

    const config = loadConfig(options.env);

    if (!options.chainName) {
        throw new Error('Chain name was not provided');
    }

    const chainName = options.chainName.toLowerCase();

    const chain = getChainConfig(config.chains, chainName);

    if (!chain) {
        throw new Error(`Chain ${chainName} is not defined in the info file`);
    }

    if (chain.chainType !== 'xrpl') {
        throw new Error(`Cannot run script for a non XRPL chain: ${chainName}`);
    }

    console.log('');
    printInfo('Chain', chain.name, chalk.cyan);

    const wallet = getWallet(options);

    const client = new XRPLClient(chain.wssRpc);
    await client.connect();

    try {
        await processor(config, wallet, client, chain, options, args);
    } catch (error) {
        printError(`Failed with error on ${chainName}`, error.message);

        if (!catchErr && !options.ignoreError) {
            throw error;
        }
    } finally {
        await client.disconnect();
    }

    if (save) {
        saveConfig(config, options.env);
    }
}

async function mainCosmosProcessor(processCmd, options) {
    const config = loadConfig(options.env);
    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);
    const {
        axelar: { gasPrice, gasLimit },
    } = config;

    const fee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));

    await processCmd(config, options, wallet, client, fee);
}

module.exports = {
    ...require('../common/utils'),
    generateWallet,
    getWallet,
    printWalletInfo,
    mainProcessor,
    mainCosmosProcessor,
    hex,
    getEvent,
    getEventAttr,
    roundUpToNearestXRP,
    deriveAddress,
    parseTokenAmount,
    decodeAccountIDToHex,
    decodeTxBlob,
    broadcastTxBlob,
};
