'use strict';

const fs = require('fs');
const { ethers } = require('hardhat');
const {
    BigNumber,
    utils: { isAddress },
} = ethers;
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

async function ledgerSign(gasLimit, gasPrice, nonce, chain, wallet, to, amount, contract, functionName, ...args) {
    const tx = {};

    if (!chain) {
        throw new Error('Chain is missing in the function arguments');
    }

    tx.chainId = chain.chainId;

    if (!gasLimit) {
        throw new Error('Gas limit is missing in the function arguments');
    }

    tx.gasLimit = gasLimit;

    if (!gasPrice) {
        throw new Error('Gas price is missing in the function arguments');
    }

    tx.gasPrice = gasPrice;

    if (!nonce) {
        throw new Error('Nonce is missing in the function arguments');
    }

    tx.nonce = nonce;

    if (!wallet) {
        throw new Error('Wallet is missing/not provided correctly in function arguments');
    }

    if (!to || !isAddress(to)) {
        throw new Error('Target address is missing/not provided as valid address for the tx in function arguments');
    }

    tx.to = to;
    tx.value = amount || undefined;

    if (contract) {
        if (to.toLowerCase() !== contract.address.toLowerCase()) {
            throw new Error('Contract address do not matches the to address provided in function arguments');
        }

        if (!functionName) {
            throw new Error('Function name is missing in the function arguments');
        }

        const data = contract.interface.encodeFunctionData(functionName, args);
        tx.data = data || undefined;
    }

    const baseTx = {
        chainId: tx.chainId || undefined,
        data: tx.data || undefined,
        gasLimit: tx.gasLimit || undefined,
        gasPrice: tx.gasPrice || undefined,
        nonce: tx.nonce ? BigNumber.from(tx.nonce).toNumber() : undefined,
        to: tx.to || undefined,
        value: tx.value || undefined,
    };
    
    let signedTx;
    try {
        signedTx = await wallet.signTransaction(baseTx);
        printInfo(`Signed Tx from ledger with signedTxHash as: ${signedTx}`);
    } catch(error) {
        printError("Failed to sign tx from ledger");
        printObj(error);
    }
    return { baseTx, signedTx };
}

function getFilePath(directoryPath, fileName) {
    if (!directoryPath) {
        throw new Error('Directory path is missing in the function arguments');
    }

    if (!fileName) {
        throw new Error('File name is missing in the function arguments');
    }

    if (!fs.existsSync(directoryPath)) {
        fs.mkdirSync(directoryPath);
    }

    const filePath = directoryPath + '/' + fileName + '.json';
    return filePath;
}

async function sendTx(tx, provider) {
    try {
        const receipt = await provider.sendTransaction(tx).then((tx) => tx.wait());
        return receipt;
    } catch (error) {
        printError('Error while broadcasting signed tx');
        printObj(error);
        return error || { error: true, message: 'Error while broadcasting signed tx' };
    }
}

async function updateSignersData(directoryPath, fileName, signersData) {
    const filePath = getFilePath(directoryPath, fileName);
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

async function getLatestNonceAndUpdateData(directoryPath, fileName, wallet) {
    try {
        const provider = wallet.provider;
        const signerAddress = await wallet.getAddress();
        const signersData = await getAllSignersData(directoryPath, fileName);
        let transactions = signersData[signerAddress];
        const firstPendingnonceFromData = getNonceFromData(transactions);
        let nonce = await getNonceFromProvider(provider, signerAddress);

        if (nonce > firstPendingnonceFromData) {
            transactions = getTxsWithUpdatedNonceAndStatus(transactions, nonce);
            signersData[signerAddress] = transactions;
            await updateSignersData(directoryPath, fileName, signersData);
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

async function getAllSignersData(directoryPath, fileName) {
    const signersData = {};

    try {
        const filePath = getFilePath(directoryPath, fileName);
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
        printError(`Failed to get all  signers data from the file ${fileName}`);
        printObj(error);
    }
}

function getFileData(filePath) {
    try {
        if (!fs.existsSync(filePath)) {
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

async function getTransactions(directoryPath, fileName, signerAddress) {
    let transactions = [];

    try {
        const filePath = getFilePath(directoryPath, fileName);
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

module.exports = {
    getLedgerWallet,
    ledgerSign,
    sendTx,
    getFilePath,
    getTxsWithUpdatedNonceAndStatus,
    getNonceFromProvider,
    getNonceFromData,
    getAllSignersData,
    getTransactions,
    updateSignersData,
    getLatestNonceAndUpdateData,
    isValidJSON,
};
