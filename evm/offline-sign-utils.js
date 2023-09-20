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

async function getNonceFromProvider(provider, address) {
    let nonce = 0;

    try {
        nonce = await provider.getTransactionCount(address);
    } catch (error) {
        printError('Could not fetch nonnce from provider', error.message);
    }

    return nonce;
}

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
        printError(`Failed to get all  signers data from the file ${filePath}`);
        printObj(error);
    }
}

function getFileData(filePath) {
    try {
        createFileIfNotExists(filePath);
        // Read the content of the file

        const data = fs.readFileSync(filePath, 'utf-8');
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

const getWallet = async (privateKey, provider, options) => {
    let wallet;

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

    try {
        fs.writeFileSync(filePath, JSON.stringify(nonceData, null, 2));
        printInfo(`Nonce updated successfully and stored in file ${filePath}`);
    } catch (err) {
        printError(`Could not update Nonce in file ${filePath}`);
        printObj(err);
    }
};

const getLocalNonce = (env, chainName, signerAddress) => {
    const nonceData = getNonceFileData();
    const chainNonceData = chainName ? nonceData[env][chainName] : undefined;
    const nonce = chainNonceData ? chainNonceData[signerAddress] || 0 : 0;
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
    getTransactions,
    storeSignedTx,
    getSignedTx,
    getWallet,
    getNonceFileData,
    updateNonceFileData,
    getLocalNonce,
    updateLocalNonce,
    ledgerSign,
    getNonceFromProvider,
};
