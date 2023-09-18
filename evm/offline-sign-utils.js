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

const { printError, printInfo, printObj } = require('./utils');

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

function getUnsignedTx(chain, tx) {
    if (!tx.to || !isAddress(tx.to)) {
        throw new Error('Target address is missing/not provided as valid address for the tx in function arguments');
    }

    const baseTx = {
        chainId: tx.chainId || chain.chainId || undefined,
        data: tx.data || undefined,
        gasLimit: tx.gasLimit || chain.gasOptions?.gasLimit || undefined,
        gasPrice: tx.gasPrice || undefined,
        nonce: tx.nonce ? BigNumber.from(tx.nonce).toNumber() : undefined,
        to: tx.to || undefined,
        value: tx.value || undefined,
    };

    return baseTx;
}

async function sendTx(tx, provider) {
    try {
        const receipt = await provider.sendTransaction(tx).then((tx) => tx.wait());
        if(!isValidJSON(receipt) || response.status !== 1) {
            const error = `Execution failed${
                response.status ? ` with txHash: ${response.transactionHash}` : ` with msg: ${response.message}`
            }`;
            throw new Error(error);
        }
        return receipt;
    } catch (errorObj) {
        printError('Error while broadcasting signed tx');
        printObj(errorObj);
    }
}

async function updateSignersData(filePath, signersData) {
    
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
    const nonce = await provider.getTransactionCount(address);
    return nonce;
}

async function getLatestNonceAndUpdateData(filePath, wallet, nonce) {
    try {
        const provider = wallet.provider;
        const signerAddress = await wallet.getAddress();
        const signersData = await getAllSignersData(filePath);
        let transactions = signersData[signerAddress];
        const firstPendingnonceFromData = getNonceFromData(transactions);
        console.log("firstPendingnonceFromData",firstPendingnonceFromData);

        if (nonce >= firstPendingnonceFromData) {
            transactions = getTxsWithUpdatedNonceAndStatus(transactions, nonce);
            signersData[signerAddress] = transactions;
            await updateSignersData(filePath, signersData);
        } else {
            nonce = firstPendingnonceFromData + 1;
        }

        return nonce;
    } catch (error) {
        printError('Failed to calculate correct nonce for tx');
        printObj(error);
    }
}

function getTxsWithUpdatedNonceAndStatus(transactions, nonce) {
    if (transactions) {
        for (const transaction of transactions) {
            if (nonce > transaction.nonce && (transaction.status === 'PENDING' || transaction.status === 'BROADCASTED')) {
                transaction.status = 'FAILED';
                const error = `Transaction nonce value of ${transaction.nonce} is less than the required signer nonce value of ${nonce}`;
                transaction.error = error;
                printError(error + ` for signedTx: ${transaction.signedTx}`);
            }
        }
    }

    return transactions;
}

function getNonceFromData(transactions) {
    try {
        if (transactions) {
            for (const transaction of transactions) {
                if (transaction.status === 'PENDING') {
                    return transaction.nonce;
                }
            }
        }
    } catch (error) {
        printError('Failed to get first pending nonce from file data');
        printObj(error);
    }

    return 0;
}

async function getAllSignersData(filePath) {
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
        if (!fs.existsSync(directoryPath)){
            fs.mkdirSync(directoryPath);
        }
        if (!fs.existsSync(filePath)) {
            console.log("Creating file");
            // File does not exist, create it
            fs.writeFileSync(filePath, JSON.stringify({}));
            return undefined;
        }
        // Read the content of the file

        const data = fs.readFileSync(filePath);
        return data;
    } catch (error) {
        printError(`Failed to get file data from the file ${filePath}`);
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

const getWallet = (privateKey, provider, ledgerPath) => {
    let wallet;
    if (privateKey === 'ledger') {
        wallet = getLedgerWallet(provider, ledgerPath || undefined);
    } else {
        if (!isValidPrivateKey(privateKey)) {
            throw new Error('Private key is missing/ not provided correctly');
        }

        wallet = new Wallet(privateKey, provider);
    }
    return wallet;
}

module.exports = {
    getLedgerWallet,
    sendTx,
    getTxsWithUpdatedNonceAndStatus,
    getNonceFromProvider,
    getNonceFromData,
    getAllSignersData,
    getTransactions,
    updateSignersData,
    getLatestNonceAndUpdateData,
    isValidJSON,
    getWallet,
    getUnsignedTx,
};
