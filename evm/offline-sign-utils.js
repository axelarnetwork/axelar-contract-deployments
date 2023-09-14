'use strict';

const {
    BigNumber,
    utils: {isAddress, serializeTransaction },
} = require('ethers');
const { LedgerSigner } = require('@ethersproject/hardware-wallets');
const {printObj, isNumber, printError } = require('./utils');
const fs = require('fs');

async function getNonce(provider, wallet) {
    const nonce = await provider.getTransactionCount(await wallet.getAddress());
    return nonce;
  }

// function to create a ledgerSigner type wallet object
function getLedgerWallet(provider, path) {
        // Check if the parameters are undefined and assign default values if necessary
        if(provider === undefined || provider === null) {
            throw new Error("Provider is not provided while creating a ledger wallet");
        }
        provider = provider
        const type = "hid";
        path = path || "m/44'/60'/0'/0/0";
        return new LedgerSigner(provider, type, path);
    }

async function ledgerSign(offline, gasLimit, gasPrice, nonce, network, chain, wallet, to, amount, contract, functionName, ...args) {
    const tx = {};
    if(offline !== "true") {
        if(network.toLowerCase() !== "mainnet" && isNumber(nonce)) {
            tx.nonce = nonce;
        }
        tx.gasLimit = gasLimit || undefined;
        tx.gasPrice = gasPrice || undefined;
    }

    if(!chain) {
        throw new Error("Chain is missing in the function arguments");
    }
    tx.chainId = chain.chainId;
    if(!wallet) {
        throw new Error("Wallet is missing/not provided correctly in function arguments");
    }
    if(!to || !isAddress(to)) {
        throw new Error("Target address is missing/not provided as valid address for the tx in funciton arguments");
    }

    tx.to = to;
    tx.value = amount || undefined;

    if(contract) {
        if(to.toLowerCase() !== contract.address.toLowerCase()) {
            throw new Error("Contract address do not matches the to address provided in function arguments");
        }
        if(!functionName) {
            throw new Error("Function name is missing in the funciton arguments");
        }
        const data = contract.interface.encodeFunctionData(functionName, args);

        tx.data = data || undefined;
        }

    const baseTx = {
      chainId: (tx.chainId || undefined),
      data: (tx.data || undefined),
      gasLimit: (tx.gasLimit || undefined),
      gasPrice: (tx.gasPrice || undefined),
      nonce: (tx.nonce ? BigNumber.from(tx.nonce).toNumber() : await getNonce(wallet)),
      to: (tx.to || undefined),
      value: (tx.value || undefined),
    };

    console.log("Printing Base obj");
    printObj(baseTx);
    if(offline !== "true") {
        return baseTx;
    }
  
    const unsignedTx = serializeTransaction(baseTx).substring(2);
    console.log("Before trying to sign using ledger wallet");
    const sig = await wallet._retry((eth) => eth.signTransaction("m/44'/60'/0'/0/0", unsignedTx));
  
    // EIP-155 sig.v computation
    // v in {0,1} + 2 * chainId + 35
    // Ledger gives this value mod 256
    // So from that, compute whether v is 0 or 1 and then add to 2 * chainId + 35 without doing a mod
    var v = BigNumber.from("0x" + sig.v).toNumber()
    v = (2 * chain.chainId + 35) + (v + 256 * 100000000000 - (2 * chain.chainId + 35)) % 256
  
    // console.log("sig v", BigNumber.from("0x" + sig.v).toNumber(), v, "chain", chainID)
    
    return serializeTransaction(baseTx, {
      v: v,
      r: ("0x" + sig.r),
      s: ("0x" + sig.s),
    });
}

async function fetchExisitingTransactions(dirPath, fileName) {
    // Read the existing transactions from the file or create a new array if the file doesn't exist
    let transactions = [];
    dirPath = dirPath || './tx';
    fileName = fileName || 'signed_transactions.txt';
    const filePath = dirPath + '/' + fileName;
    try {
    const existingData = fs.readFileSync(filePath);
    transactions = JSON.parse(existingData).transactions;
    } catch (error) {
        printError("File doesn't exist yet, that's fine");
    }
    return transactions;
}

function updateTransactions(transactions, msg, signedTransaction) {
    transactions.push({ "msg": msg, "signedTransaction": signedTransaction });
    return transactions;
}

async function storeTransactionsData(dirPath, fileName, msg, signedTransaction) {
    let transactions = await fetchExisitingTransactions(dirPath, fileName);
    transactions = updateTransactions(transactions, msg, signedTransaction);
    dirPath = dirPath || './tx';
    if (!fs.existsSync(dirPath)) {
        fs.mkdirSync(dirPath);
      }
    fileName = fileName || 'signed_transactions.txt';
    const filePath = dirPath + '/' + fileName;
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
    console.log("command", receipt);
  }

  async function getNonce(wallet) {
    const provider = wallet.provider;
    const nonce = await provider.getTransactionCount(await wallet.getAddress());
    return nonce;
  }
  

module.exports = {
    getUnsignedTx,
    getNonce,
    getLedgerWallet,
    ledgerSign,
    sendTx,
    storeTransactionsData
}