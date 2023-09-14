'use strict';

const {
    BigNumber,
    utils: { isAddress, serializeTransaction },
} = require('ethers');
const { LedgerSigner } = require('@ethersproject/hardware-wallets');
const { printObj, isNumber, printError } = require('./utils');
const fs = require('fs');


// function to create a ledgerSigner type wallet object
function getLedgerWallet(provider, path) {
    // Check if the parameters are undefined and assign default values if necessary
    if (provider === undefined || provider === null) {
        throw new Error('Provider is not provided while creating a ledger wallet');
    }
    provider = provider;
    const type = 'hid';
    path = path || "m/44'/60'/0'/0/0";
    return new LedgerSigner(provider, type, path);
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
        throw new Error('Target address is missing/not provided as valid address for the tx in funciton arguments');
    }

    tx.to = to;
    tx.value = amount || undefined;

    if (contract) {
        if (to.toLowerCase() !== contract.address.toLowerCase()) {
            throw new Error('Contract address do not matches the to address provided in function arguments');
        }
        if (!functionName) {
            throw new Error('Function name is missing in the funciton arguments');
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


    const signedTx = await wallet.signTransaction(baseTx);
    return [baseTx, signedTx];
}

async function fetchExisitingTransactions(directoryPath, fileName) {
    // Read the existing transactions from the file or create a new array if the file doesn't exist
    let transactions = [];
    const filePath = getFilePath(directoryPath, fileName);
    try {
        const existingData = getFileData(filePath);
        transactions = JSON.parse(existingData).transactions;
    } catch (error) {
        printError("File doesn't exist yet, that's fine");
    }
    return transactions;
}

function getFilePath(directoryPath, fileName) {
    if(!directoryPath) {
        throw new Error("Directory path is missing in the function arguments");
    }
    if(!fileName) {
        throw new Error("File name is missing in the function arguments");
    }
    if (!fs.existsSync(directoryPath)) {
        fs.mkdirSync(directoryPath);
    }
    const filePath = directoryPath + '/' + fileName + '.json';
    return filePath;
}

function updateTransactions(transactions, msg, signedTransaction) {
    transactions.push({ msg: msg, signedTransaction: signedTransaction });
    return transactions;
}

async function storeTransactionsData(directoryPath, fileName, msg, signedTransaction) {
    let transactions = await fetchExisitingTransactions(directoryPath, fileName);
    transactions = updateTransactions(transactions, msg, signedTransaction);
    directoryPath = directoryPath || './tx';
    if (!fs.existsSync(directoryPath)) {
        fs.mkdirSync(directoryPath);
    }
    fileName = fileName || 'signed_transactions.txt';
    const filePath = directoryPath + '/' + fileName;
    const data = JSON.stringify({ transactions }, null, 2);
    fs.writeFileSync(filePath, data, (err) => {
        if (err) {
            printError(err);
            return;
        }
        print(`Data has been successfully stored in the ${filePath} file.`);
    });
}

async function sendTx(tx, provider) {
    const receipt = await provider.sendTransaction(tx).then((tx) => tx.wait());
    return receipt;
}

async function updateSignersData(directoryPath, fileName, signersData) {
    const filePath = getFilePath(directoryPath, fileName);
    fs.writeFileSync(filePath, JSON.stringify(signersData, null, 2), (err) => {
        if (err) {
            printError(err);
            return;
        }
        print(`Data has been successfully stored in the ${filePath} file.`);
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
        let signersData = getAllSignersData(directoryPath, fileName);
        let signerData = signersData[signerAddress];
        const nonceFromData = getLatestNonceFromData(signerData); 
        let nonce = await getNonceFromProvider(provider, signerAddress);
        if(nonce > nonceFromData) {
            signerData = updateTxNonceAndStatus(signerData, nonce);
            signersData[signerAddress] = signerData;
            await updateSignersData(directoryPath, fileName, signersData);
        }
        else {
            nonce = nonceFromData + 1;
        }
        return nonce;

    } catch(error) {
        printError(error.message);
    }
}

function updateTxNonceAndStatus(signerData, nonce) {
    if(signerData) {
        for(const transaction of signerData) {
            if(nonce > transaction.nonce && (transaction.status === "PENDING" || transaction.status === "BROADCASTED")) {
                transaction.status = "FAILED";
                transaction.error = `Transaction nonce value of ${transaction.nonce} is less than the required signer nonce value of ${nonce}`;
            }
        }
    }
    return signerData
}

function getLatestNonceFromData(signerData) {
    if(signerData) {
        const length = signerData.length;
        return parseInt(signerData[length - 1].nonce);
    }
    return 0;
}

async function getAllSignersData(directoryPath, fileName) {
    const signersData = {};
    try {
        const filePath = getFilePath(directoryPath, fileName);
        // Read the content of the file
        const data = getFileData(filePath);
        if(data) {
            const jsonData = JSON.parse(data);
            if(!isValidJSON(jsonData)) {
                return signersData;
            }
            return jsonData;
        }
        return signersData;
    } catch(error) {
        printError(error.message);
    }
}

function getFileData(filePath) {
    try {
        if (!fs.existsSync(filePath)) {
            // File does not exist, create it
            fs.writeFileSync(filePath, {});
            return undefined;
        }
        // Read the content of the file
        const data = fs.readFileSync(filePath);
        return data;
    } catch(error) {
        printError(error.message);
    }
}

async function getSignerData(directoryPath, fileName, signerAddress) {
    let signerData = [];
    console.log(fileName);
    try {
        const filePath = getFilePath(directoryPath, fileName);
        // Read the content of the file
        const data = getFileData(filePath);
        console.log(data);
        if(data) {
            const jsonData = JSON.parse(data);
            console.log(jsonData);
            if(!isValidJSON(jsonData)) {
                return signerData;
            }
            // Access the transactions array from the JSON object
            if(signerAddress in jsonData) {
                console.log("line 245");
                signerData = jsonData[signerAddress];
            }
        }
        return signerData;

    } catch(error) {
        printError(error.message);
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
    storeTransactionsData,
    getFilePath,
    updateTxNonceAndStatus,
    getNonceFromProvider,
    getLatestNonceFromData,
    getAllSignersData,
    getSignerData,
    updateSignersData,
    getLatestNonceAndUpdateData
};
