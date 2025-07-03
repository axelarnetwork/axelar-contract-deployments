const { TonClient, WalletContractV5R1, internal, Cell } = require('@ton/ton');
const { mnemonicToWalletKey } = require('@ton/crypto');
const { getEmptySignature } = require('axelar-cgp-ton');
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
        await new Promise((resolve) => setTimeout(resolve, 1500));
        currentSeqno = await contract.getSeqno();
    }
    console.log('Transaction confirmed!');
}

function parseWeightedSigners(jsonString) {
    try {
        const parsed = JSON.parse(jsonString);

        // Validate structure
        if (!parsed.signers || !Array.isArray(parsed.signers)) {
            throw new Error('Invalid format: signers must be an array');
        }

        if (typeof parsed.threshold === 'undefined' || typeof parsed.nonce === 'undefined') {
            throw new Error('Invalid format: threshold and nonce are required');
        }

        // Convert to proper types
        const weightedSigners = {
            signers: parsed.signers.map((signer) => ({
                signer: BigInt(signer.signer),
                weight: BigInt(signer.weight),
                signature: getEmptySignature(),
            })),
            threshold: BigInt(parsed.threshold),
            nonce: BigInt(parsed.nonce),
        };

        return weightedSigners;
    } catch (error) {
        if (error instanceof SyntaxError) {
            throw new Error('Invalid JSON format');
        }
        throw error;
    }
}

async function sendTransactionWithCost(contract, key, gateway, messageBody, cost) {
    const message = internal({
        to: gateway,
        value: cost,
        body: messageBody,
    });

    const seqno = await contract.getSeqno();
    console.log('Current wallet seqno:', seqno);

    const transfer = await contract.sendTransfer({
        secretKey: key.secretKey,
        messages: [message],
        seqno: seqno,
    });

    return { transfer, seqno };
}

function bocToCell(encodedPayload) {
    return Cell.fromBoc(Buffer.from(encodedPayload, 'hex'))[0];
}

module.exports = {
    sendTransactionWithCost,
    getTonClient,
    parseWeightedSigners,
    loadWallet,
    waitForTransaction,
    bocToCell,
    TONCENTER_ENDPOINT,
    GATEWAY_ADDRESS,
};
