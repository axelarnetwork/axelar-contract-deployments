'use strict';

const { verifyTransactionBlock } = require('@mysten/sui.js/verify');
const { decodeSuiPrivateKey } = require('@mysten/sui.js/cryptography');
const { Ed25519Keypair } = require('@mysten/sui.js/keypairs/ed25519');
const { Secp256k1Keypair } = require('@mysten/sui.js/keypairs/secp256k1');
const { Secp256r1Keypair } = require('@mysten/sui.js/keypairs/secp256r1');
const { printInfo } = require('../evm/utils');
const { SuiClient, getFullnodeUrl } = require('@mysten/sui.js/client');

function getWallet(chain, options) {
    let keypair;
    let scheme;

    switch (options.signatureScheme) {
        case 'ed25519': {
            scheme = Ed25519Keypair;
            break;
        }

        case 'secp256k1': {
            scheme = Secp256k1Keypair;
            break;
        }

        case 'secp256r1': {
            scheme = Secp256r1Keypair;
            break;
        }

        default: {
            throw new Error(`Unsupported signature scheme: ${options.signatureScheme}`);
        }
    }

    switch (options.privateKeyType) {
        case 'bech32': {
            const decodedKey = decodeSuiPrivateKey(options.privateKey);
            const secretKey = decodedKey.secretKey;
            keypair = scheme.fromSecretKey(secretKey);
            break;
        }

        case 'mnemonic': {
            keypair = scheme.deriveKeypair(options.privateKey);
            break;
        }

        case 'hex': {
            const privKey = Buffer.from(options.privateKey, 'hex');
            keypair = scheme.fromSecretKey(privKey);
            break;
        }

        default: {
            throw new Error(`Unsupported key type: ${options.privateKeyType}`);
        }
    }

    const url = chain.rpc || getFullnodeUrl(chain.networkType);
    const client = new SuiClient({ url });

    return [keypair, client];
}

async function printWalletInfo(keypair, client, chain, options) {
    printInfo('Wallet address', keypair.toSuiAddress());

    const coins = await client.getBalance({ owner: keypair.toSuiAddress() });
    printInfo('Wallet balance', `${coins.totalBalance / 1e9} ${chain.tokenSymbol || coins.coinType}`);
}

async function generateKeypair(options) {
    switch (options.signatureScheme) {
        case 'ed25519':
            return Ed25519Keypair.generate();
        case 'secp256k1':
            return Secp256k1Keypair.generate();
        case 'secp256r1':
            return Secp256r1Keypair.generate();

        default: {
            throw new Error(`Unsupported scheme: ${options.signatureScheme}`);
        }
    }
}

function getRawPrivateKey(keypair) {
    return decodeSuiPrivateKey(keypair.getSecretKey()).secretKey;
}

async function broadcast(client, keypair, tx) {
    return await client.signAndExecuteTransactionBlock({
        transactionBlock: tx,
        signer: keypair,
        options: {
            showEffects: true,
            showObjectChanges: true,
            showContent: true,
        },
    });
}

async function signTransactionBlock(chain, txDetails, options) {
    const { txBlock, buildOptions = {} } = txDetails;

    const [keypair, client] = getWallet(chain, options);
    txBlock.setSenderIfNotSet(keypair.toSuiAddress());
    const txBytes = await txBlock.build(buildOptions);
    const serializedSignature = (await keypair.signTransactionBlock(txBytes)).signature;
    let publicKey;

    try {
        publicKey = await verifyTransactionBlock(txBytes, serializedSignature);
    } catch {
        throw new Error(`Cannot verify tx signature`);
    }

    if (publicKey.toSuiAddress() !== keypair.toSuiAddress()) {
        throw new Error(`Verification failed for address ${keypair.toSuiAddress()}`);
    }

    if (!options.offline) {
        const txResult = await client.executeTransactionBlock({
            transactionBlock: txBytes,
            signature: serializedSignature,
        });

        printInfo('Transaction result', JSON.stringify(txResult));
    }

    return {
        signature: serializedSignature,
        txBlock,
        publicKey,
    };
}

module.exports = {
    getWallet,
    printWalletInfo,
    generateKeypair,
    getRawPrivateKey,
    broadcast,
    signTransactionBlock,
};
