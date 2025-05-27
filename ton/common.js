// ton/common.js
const { TonClient, WalletContractV5R1 } = require('@ton/ton');
const { mnemonicToWalletKey } = require('@ton/crypto');
require('dotenv').config();

// Constants
const TONCENTER_ENDPOINT = 'https://testnet.toncenter.com/api/v2/jsonRPC';
const GATEWAY_ADDRESS = process.env.TON_GATEWAY_ADDRESS;

if (!GATEWAY_ADDRESS) {
    throw new Error('Please set TON_GATEWAY_ADDRESS in your .env file');
}

// Helper function to initialize TON client
function getTonClient() {
    if (!process.env.TONCENTER_API_KEY) {
        throw new Error('Please set TONCENTER_API_KEY environment variable. Get it from https://t.me/tontestnetapibot');
    }

    return new TonClient({
        endpoint: TONCENTER_ENDPOINT,
        apiKey: process.env.TONCENTER_API_KEY,
    });
}

// Helper function to load wallet
async function loadWallet(client) {
    const mnemonic = process.env.MNEMONIC?.split(' ') || [];
    if (mnemonic.length !== 24) {
        throw new Error('Please set MNEMONIC environment variable with 24 words');
    }

    const key = await mnemonicToWalletKey(mnemonic);
    const wallet = WalletContractV5R1.create({ publicKey: key.publicKey, workchain: 0 });
    return { contract: client.open(wallet), key, wallet };
}

// Helper function to wait for transaction confirmation
async function waitForTransaction(contract, seqno) {
    let currentSeqno = seqno;
    while (currentSeqno === seqno) {
        console.log('Waiting for transaction confirmation...');
        await new Promise(resolve => setTimeout(resolve, 1500));
        currentSeqno = await contract.getSeqno();
    }
    console.log('Transaction confirmed!');
}

module.exports = {
    getTonClient,
    loadWallet,
    waitForTransaction,
    TONCENTER_ENDPOINT,
    GATEWAY_ADDRESS,
};
