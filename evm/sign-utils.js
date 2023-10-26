'use strict';

const fs = require('fs');
const { ethers } = require('hardhat');
const {
    Wallet,
    utils: { isAddress },
} = ethers;

const path = require('path');
const { LedgerSigner } = require('./LedgerSigner');

const { printError, printInfo, printObj, isValidPrivateKey, isNumber, isValidNumber } = require('./utils');

/**
 * Get a wallet object from a private key or a ledger device
 * @param {*} privateKey - private key or 'ledger'
 * @param {*} provider - provider object
 * @param {*} options - options object. ledgerPath can be provided for custom HD derivation
 * @returns
 */
const getWallet = async (privateKey, provider, options = {}) => {
    let wallet;

    if (options.offline) {
        provider = undefined;
    }

    if (privateKey === 'ledger') {
        wallet = await getLedgerWallet(provider, options?.ledgerPath);
    } else {
        if (!isValidPrivateKey(privateKey)) {
            throw new Error('Private key is missing/ not provided correctly');
        }

        wallet = new Wallet(privateKey, provider);
    }

    return wallet;
};

// function to create a ledgerSigner type wallet object
const getLedgerWallet = async (provider, path) => {
    path = path || "m/44'/60'/0'/0/0";

    return new LedgerSigner(provider, path);
};

/**
 * Sign a transaction with a wallet. Supports offline mode, and a private key or ledger backend
 * @param {*} wallet - Either private key or ledger wallet
 * @param {*} chain - chain config
 * @param {*} tx - unsigned base transaction
 * @param {*} options
 * @returns - unsigned and signed transaction
 */
const signTransaction = async (wallet, chain, tx, options = {}) => {
    if (!tx.to || !isAddress(tx.to)) {
        throw new Error('Target address is missing/not provided as valid address for the tx in function arguments');
    }

    if (options.gasOptions) {
        tx = {
            ...options.gasOptions,
            ...tx, // prefer gas options from tx if they were set
        };
    }

    if (!options.offline) {
        tx = await wallet.populateTransaction(tx);
    } else {
        const address = options.signerAddress || (await wallet.getAddress());

        tx = {
            ...chain.staticGasOptions,
            chainId: chain.chainId,
            nonce: options.nonce,
            from: address,
            ...tx, // prefer tx options if they were set
        };

        if (tx.nonce === undefined) {
            tx.nonce = getLocalNonce(options.env, chain.name.toLowerCase(), address);

            if (tx.nonce === undefined) {
                throw new Error(`Nonce is missing for ${address} on ${chain.name} in nonces.json`);
            }
        }

        if (options.nonceOffset) {
            if (!isValidNumber(options.nonceOffset)) {
                throw new Error('Provided nonce offset is not a valid number');
            }

            tx.nonce += parseInt(options.nonceOffset);
        }

        if (!tx.gasLimit) {
            throw new Error('Gas limit is missing/not provided for the tx in function arguments');
        }

        if (
            !tx.gasPrice &&
            !(isValidNumber(tx.maxFeePerGas) && (tx.maxPriorityFeePerGas === undefined || isNumber(tx.maxPriorityFeePerGas)))
        ) {
            throw new Error('Gas price (legacy or eip-1559) is missing/not provided for the tx in function arguments');
        }

        if (tx.maxFeePerGas !== undefined) {
            tx.type = 2;
        } else {
            tx.type = 0;
        }

        printInfo('Transaction being signed', JSON.stringify(tx, null, 2));
    }

    const signedTx = await wallet.signTransaction(tx);

    if (!options.offline) {
        await sendTransaction(signedTx, wallet.provider, chain.confirmations);
    }

    return { baseTx: tx, signedTx };
};

const sendTransaction = async (tx, provider, confirmations = undefined) => {
    const response = await provider.sendTransaction(tx);
    const receipt = await response.wait(confirmations);

    printInfo('Broadcasted tx', response.hash);

    return { response, receipt };
};

function storeSignedTx(filePath, signedTx) {
    createFileIfNotExists(filePath);
    fs.writeFileSync(filePath, JSON.stringify(signedTx, null, 2), (err) => {
        if (err) {
            printError(`Could not store signedTx in file ${filePath}`);
            printObj(err);
            return;
        }

        printInfo(`Data has been successfully stored in the ${filePath} file.`);
    });
}

const getNonceFromProvider = async (provider, address) => {
    return await provider.getTransactionCount(address);
};

function getSignedTx(filePath) {
    const signedTx = {};

    try {
        // Read the content of the file
        const data = getFileData(filePath);

        if (data) {
            const jsonData = JSON.parse(data);

            if (!isValidJSON(jsonData)) {
                return signedTx;
            }

            return jsonData;
        }

        return signedTx;
    } catch (error) {
        printError(`Failed to get all signers data from the file ${filePath}`, error);
        throw error;
    }
}

function getFileData(filePath) {
    try {
        createFileIfNotExists(filePath);
        // Read the content of the file

        const data = fs.readFileSync(filePath, 'utf-8');
        return data;
    } catch (error) {
        printError(`Failed to get data from the file ${filePath}`, error);
        throw error;
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
        printError(`Failed to get transactions for ${signerAddress}`, error);
        throw error;
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

const getNonceFileData = () => {
    const filePath = `${__dirname}/nonces.json`;
    const emptyData = {};
    const data = getFileData(filePath);

    if (data) {
        const jsonData = JSON.parse(data);

        if (!isValidJSON(jsonData)) {
            return emptyData;
        }

        return jsonData;
    }

    return emptyData;
};

function createFileIfNotExists(filePath) {
    const directoryPath = path.dirname(filePath);

    // Check if the directory and file exists, create it if it doesn't
    if (!fs.existsSync(directoryPath)) {
        fs.mkdirSync(directoryPath, { recursive: true }); // Added { recursive: true } to create parent directories if needed
    }

    if (!fs.existsSync(filePath)) {
        // File does not exist, create it
        fs.writeFileSync(filePath, JSON.stringify({}, null, 2));
    }
}

const updateNonceFileData = (nonceData) => {
    const filePath = `${__dirname}/nonces.json`;
    createFileIfNotExists(filePath);

    // Write nonceData to the file
    fs.writeFileSync(filePath, JSON.stringify(nonceData, null, 2));
    printInfo(`Nonce updated successfully and stored in file ${filePath}`);
};

const getLocalNonce = (env, chainName, signerAddress) => {
    const nonceData = getNonceFileData();
    return nonceData[env][chainName][signerAddress];
};

const updateLocalNonce = (chain, nonce, signerAddress) => {
    const nonceData = chain.nonceData || {};
    nonceData[signerAddress] = nonce;
    chain.nonceData = nonceData;
    return chain;
};

module.exports = {
    sendTransaction,
    getTransactions,
    storeSignedTx,
    getSignedTx,
    getWallet,
    getNonceFileData,
    updateNonceFileData,
    getLocalNonce,
    updateLocalNonce,
    signTransaction,
    getNonceFromProvider,
};
