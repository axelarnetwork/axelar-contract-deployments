'use strict';

const xrpl = require('xrpl');
const chalk = require('chalk');
const {
    loadConfig,
    saveConfig,
    printInfo,
    printWarn,
    printError,
} = require('../common');

const KEY_TYPE = xrpl.ECDSA.secp256k1;

const hex = (str) => Buffer.from(str).toString('hex');

function roundUpToNearestXRP(amountInDrops) {
    return Math.ceil(amountInDrops / 1e6) * 1e6;
}

async function getAccountInfo(client, address) {
    try {
        const accountInfoRes = await client.request({
            command: 'account_info',
            account: address,
            ledger_index: 'validated',
        });

        const accountInfo = accountInfoRes.result.account_data;
        return {
            balance: accountInfo.Balance,
            sequence: accountInfo.Sequence,
        }
    } catch (error) {
        if (error.data.error === 'actNotFound') {
            return {
                balance: '0',
                sequence: '-1',
            }
        }

        printError('Failed to get account info for wallet', address);
        throw error;
    }
}

async function getReserveRequirements(client) {
    const serverInfoRes = await client.request({ command: 'server_info' });
    const validatedLedger = serverInfoRes.result.info.validated_ledger;
    return {
        baseReserve: validatedLedger.reserve_base_xrp,
        ownerReserve: validatedLedger.reserve_inc_xrp,
    }
}

async function getFee(client) {
    const feeRes = await client.request({ command: 'fee' });
    return feeRes.result.drops.open_ledger_fee;
}

async function printWalletInfo(client, wallet, chain) {
    const address = wallet.address;
    const { balance, sequence } = await getAccountInfo(client, address);
    printInfo('Wallet address', address);
    printInfo('Wallet balance', `${xrpl.dropsToXrp(balance)} ${chain.tokenSymbol || ''}`);

    if (sequence === -1) {
        printWarn('Wallet is not active because it does not meet the base reserve requirement');
    } else {
        printInfo('Wallet sequence', sequence);
    }
}

function generateWallet() {
    return xrpl.Wallet.generate(KEY_TYPE);
}

function getWallet(options) {
    return xrpl.Wallet.fromSeed(options.privateKey);
}

async function sendTransaction(client, signer, tx) {
    printInfo('Sending transaction', JSON.stringify(tx, null, 2));
    const prepared = await client.autofill(tx);
    const signed = signer.sign(prepared);

    const receipt = await client.submitAndWait(signed.tx_blob);

    if (receipt.result.meta.TransactionResult !== 'tesSUCCESS') {
        printError('Transaction failed', receipt.result);
        throw new Error(`Transaction failed: ${receipt.result.meta.TransactionResult}`);
    }

    printInfo('Transaction sent');
    return receipt;
}

async function sendPayment(client, wallet, args) {
    const { destination, amount, memos = [], fee = undefined } = args;
    const paymentTx = {
        TransactionType: 'Payment',
        Account: wallet.address,
        Destination: destination,
        Amount: amount,
        Fee: fee,
        Memos: memos.map((memo) => ({
            Memo: {
                MemoType: memo.memoType,
                MemoData: memo.memoData,
            },
        })),
    };

    return await sendTransaction(client, wallet, paymentTx);
}

async function sendSignerListSet(client, wallet, args) {
    const { quorum, signers } = args;
    const signerListSetTx = {
        TransactionType: 'SignerListSet',
        Account: wallet.address,
        SignerQuorum: quorum,
        SignerEntries: signers.map((signer) => ({
            SignerEntry: {
                Account: signer.address,
                SignerWeight: signer.weight,
            },
        })),
    };

    return await sendTransaction(client, wallet, signerListSetTx);
}

async function sendTicketCreate(client, wallet, args) {
    const { ticketCount } = args;
    const ticketCreateTx = {
        TransactionType: 'TicketCreate',
        Account: wallet.address,
        TicketCount: ticketCount,
    };

    return await sendTransaction(client, wallet, ticketCreateTx);
}

async function sendAccountSet(client, wallet, args) {
    const { transferRate, tickSize, domain, flags } = args;
    const accountSetTx = {
        TransactionType: 'AccountSet',
        Account: wallet.address,
        TransferRate: transferRate,
        TickSize: tickSize,
        Domain: domain,
        SetFlag: flags,
    };

    return await sendTransaction(client, wallet, accountSetTx);
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

    const client = new xrpl.Client(chain.wssRpc);

    try {
        await client.connect();
        await processCommand(config, chain, client, options);
    } catch (error) {
        printError(`Failed with error on ${chain.name}`, error.message);

        if (!catchErr && !options.ignoreError) {
            throw error;
        }
    } finally {
        await client.disconnect();
    }

    if (save) {
        saveConfig(config, options.env);
    }
};

module.exports = {
    ...require('../common/utils'),
    generateWallet,
    getWallet,
    getAccountInfo,
    getReserveRequirements,
    printWalletInfo,
    getFee,
    mainProcessor,
    sendTransaction,
    sendPayment,
    sendSignerListSet,
    sendAccountSet,
    sendTicketCreate,
    hex,
    roundUpToNearestXRP,
};
