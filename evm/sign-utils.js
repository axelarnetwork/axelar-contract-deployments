'use strict';

const fs = require('fs');
const { ethers } = require('hardhat');
const {
    Wallet,
    BigNumber,
    utils: { isAddress, serializeTransaction },
} = ethers;

const path = require('path');
const { LedgerSigner } = require('@ethersproject/hardware-wallets');

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
        wallet = getLedgerWallet(provider, options?.ledgerPath);
    } else {
        if (!isValidPrivateKey(privateKey)) {
            throw new Error('Private key is missing/ not provided correctly');
        }

        wallet = new Wallet(privateKey, provider);
    }

    return wallet;
};

// function to create a ledgerSigner type wallet object
const getLedgerWallet = (provider, path) => {
    const type = 'hid';
    path = path || "m/44'/60'/0'/0/0";
    return new LedgerSigner(provider, type, path);
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
        // force legacy tx type for ledger signer
        if (wallet instanceof LedgerSigner) {
            tx.type = 0;
            tx.gasPrice = tx.gasPrice || (await wallet.provider.getGasPrice());
        }

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

        if (!tx.nonce) {
            tx.nonce = getLocalNonce(options.env, chain.name.toLowerCase(), address);
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

        if (!tx.gasPrice && !(isNumber(tx.maxFeePerGas) && isNumber(tx.maxPriorityFeePerGas))) {
            throw new Error('Gas price (legacy or eip-1559) is missing/not provided for the tx in function arguments');
        }

        printInfo('Transaction being signed', JSON.stringify(tx, null, 2));
    }

    let signedTx;

    if (wallet instanceof LedgerSigner) {
        // Ledger doesn't like .from to be set
        delete tx.from;

        signedTx = await ledgerSign(wallet, chain, tx);
    } else {
        signedTx = await wallet.signTransaction(tx);
    }

    if (!options.offline) {
        await sendTransaction(signedTx, wallet.provider);
    }

    return { baseTx: tx, signedTx };
};

const ledgerSign = async (wallet, chain, baseTx) => {
    printInfo('Waiting for user to approve transaction through ledger wallet');

    const unsignedTx = serializeTransaction(baseTx).substring(2);
    const sig = await wallet._retry((eth) => eth.signTransaction("m/44'/60'/0'/0/0", unsignedTx));

    // EIP-155 sig.v computation
    // v in {0,1} + 2 * chainId + 35
    // Ledger gives this value mod 256
    // So from that, compute whether v is 0 or 1 and then add to 2 * chainId + 35 without doing a mod
    var v = BigNumber.from('0x' + sig.v).toNumber();
    v = 2 * chain.chainId + 35 + ((v + 256 * 100000000000 - (2 * chain.chainId + 35)) % 256);

    const signedTx = serializeTransaction(baseTx, {
        v,
        r: '0x' + sig.r,
        s: '0x' + sig.s,
    });

    printInfo('Signed Tx from ledger with signedTxHash as', signedTx);

    return signedTx;
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
    const filePath = `${__dirname}/../axelar-chains-config/info/nonces.json`;
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
    const filePath = `${__dirname}/../axelar-chains-config/info/nonces.json`;
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
