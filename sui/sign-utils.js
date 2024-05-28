'use strict';

const { Ed25519Keypair } = require('@mysten/sui.js/keypairs/ed25519');
const { Secp256k1Keypair } = require('@mysten/sui.js/keypairs/secp256k1');
const { Secp256r1Keypair } = require('@mysten/sui.js/keypairs/secp256r1');
const { printInfo } = require('../evm/utils');

async function getWallet(chain, options) {
    const privKey = Buffer.from(options.privateKey, 'hex');

    let keypair;

    switch (options.privateKeyType) {
        case 'ed25519': {
            keypair = Ed25519Keypair.fromSecretKey(privKey);
            break;
        }

        case 'secp256k1': {
            keypair = Secp256k1Keypair.fromSecretKey(privKey);
            break;
        }

        case 'secp256r1': {
            keypair = Secp256r1Keypair.fromSecretKey(privKey);
            break;
        }

        default: {
            throw new Error(`Unsupported key type: ${options.privateKeyType}`);
        }
    }

    printInfo('Wallet address', keypair.toSuiAddress());

    return keypair;
}

module.exports = {
    getWallet,
};
