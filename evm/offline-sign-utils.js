'use strict';

const fs = require('fs');
const { ethers } = require('hardhat');
const {
    Wallet,
    BigNumber,
    utils: { isAddress },
} = ethers;
const path = require('path');
const { LedgerSigner } = require('@ethersproject/hardware-wallets');

const { printError, printInfo, printObj, isValidPrivateKey } = require('./utils');

// function to create a ledgerSigner type wallet object
function getLedgerWallet(provider, path) {
    try {
        // Check if the parameters are undefined and assign default values if necessary
        if (provider === undefined || provider === null) {
            throw new Error('Empty provider');
        }

        const type = 'hid';
        path = path || "m/44'/60'/0'/0/0";
        return new LedgerSigner(provider, type, path);
    } catch (error) {
        printError('Error trying to coonect to ledger wallet');
        printObj(error);
    }
}

async function ledgerSign(wallet, chain, tx, gasOptions) {
    if (!tx.to || !isAddress(tx.to)) {
        throw new Error('Target address is missing/not provided as valid address for the tx in function arguments');
    }

    if (gasOptions) {
        tx.gasLimit = gasOptions.gasLimit;
        tx.gasPrice = gasOptions.gasPrice;
    }

    const baseTx = {
        chainId: tx.chainId || chain.chainId || undefined,
        data: tx.data || undefined,
        gasLimit: tx.gasLimit || chain.gasOptions?.gasLimit || undefined,
        gasPrice: tx.gasPrice || undefined,
        nonce: tx.nonce !== undefined && tx.nonce !== null ? BigNumber.from(tx.nonce).toNumber() : undefined,
        to: tx.to || undefined,
        value: tx.value || undefined,
    };

    let signedTx;

    try {
        signedTx = await wallet.signTransaction(baseTx);
        printInfo('Signed Tx from ledger with signedTxHash as', signedTx);
    } catch (error) {
        printError('Failed to sign tx from ledger');
        printObj(error);
    }

    return { baseTx, signedTx };
}

async function sendTx(tx, provider) {
    let success;

    try {
        const response = await provider.sendTransaction(tx).then((tx) => tx.wait());

        if (response.error || !isValidJSON(response) || response.status !== 1) {
            const error = `Execution failed${
                response.status ? ` with txHash: ${response.transactionHash}` : ` with msg: ${response.message}`
            }`;
            throw new Error(error);
        }

        success = true;
        return { success, response };
    } catch (errorObj) {
        printError('Error while broadcasting signed tx');
        printObj(errorObj);
        success = false;
        return { success, undefined };
    }
}

function updateSignersData(filePath, signersData) {
    fs.writeFileSync(filePath, JSON.stringify(signersData, null, 2), (err) => {
        if (err) {
            printError(`Could not update signersData in file ${filePath}`);
            printObj(err);
            return;
        }

        printInfo(`Data has been successfully stored in the ${filePath} file.`);
    });
}

async function getNonceFromProvider(provider, address) {
    let nonce = 0;

    try {
        nonce = await provider.getTransactionCount(address);
    } catch (error) {
        printError('Could not fetch nonnce from provider', error.message);
    }

    return nonce;
}

function getAllSignersData(filePath) {
    const signersData = {};

    try {
        // Read the content of the file
        const data = getFileData(filePath);

        if (data) {
            const jsonData = JSON.parse(data);

            if (!isValidJSON(jsonData)) {
                return signersData;
            }

            return jsonData;
        }

        return signersData;
    } catch (error) {
        printError(`Failed to get all  signers data from the file ${filePath}`);
        printObj(error);
    }
}

function getFileData(filePath) {
    try {
        // Extract the directory path
        const directoryPath = path.dirname(filePath);
        // Check if the directory and file exists, create it if it doesn't

        if (!fs.existsSync(directoryPath)) {
            fs.mkdirSync(directoryPath);
        }

        if (!fs.existsSync(filePath)) {
            // File does not exist, create it
            fs.writeFileSync(filePath, JSON.stringify({}));
            return undefined;
        }
        // Read the content of the file

        const data = fs.readFileSync(filePath);
        return data;
    } catch (error) {
        printError(`Failed to get data from the file ${filePath}`);
        printObj(error);
    }
}

async function getTransactions(filePath, signerAddress) {
    let transactions = [];

    try {
        // Read the content of the file
        const data = getFileData(filePath);

        if (data) {
            const jsonData = JSON.parse(data);

            if (!isValidJSON(jsonData)) {
                return transactions;
            }
            // Access the transactions array from the JSON object

            if (signerAddress in jsonData) {
                transactions = jsonData[signerAddress];
            }
        }

        return transactions;
    } catch (error) {
        printError(`Failed to get transactions for ${signerAddress}`);
        printObj(error);
    }
}

function isValidJSON(obj) {
    if (obj === undefined || obj === null) {
        return false;
    }

    if (Object.keys(obj).length === 0 && obj.constructor === Object) {
        return false;
    }

    return true;
}

const getWallet = async (privateKey, provider, ledgerPath) => {
    let wallet;

    if (privateKey === 'ledger') {
        wallet = getLedgerWallet(provider, ledgerPath || undefined);
    } else {
        if (!isValidPrivateKey(privateKey)) {
            throw new Error('Private key is missing/ not provided correctly');
        }

        wallet = new Wallet(privateKey, provider);
    }

    const signerAddress = await wallet.getAddress();
    const providerNonce = await getNonceFromProvider(provider, signerAddress);
    return { wallet, providerNonce };
};

const getLocalNonce = (chain, signerAddress) => {
    const nonceData = chain ? chain.nonceData : undefined;
    const nonce = nonceData ? nonceData[signerAddress] || 0 : 0;
    return nonce;
};

const updateLocalNonce = (chain, nonce, signerAddress) => {
    const nonceData = chain.nonceData || {};
    nonceData[signerAddress] = nonce;
    chain.nonceData = nonceData;
    return chain;
};

module.exports = {
    sendTx,
    getAllSignersData,
    getTransactions,
    updateSignersData,
    getWallet,
    getLocalNonce,
    updateLocalNonce,
    ledgerSign,
};
