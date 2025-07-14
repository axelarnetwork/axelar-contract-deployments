'use strict';

const { generateSecretKey, generateWallet } = require('@stacks/wallet-sdk');
const { privateKeyToAddress } = require('@stacks/transactions');

async function getWallet(chain, options) {
    if (!options.mnemonic && !options.privateKey) {
        throw new Error('Mnemonic or private key is required');
    }
    if (options.mnemonic && options.privateKey) {
        throw new Error('Can only use one of Stacks mnemonic or private key');
    }
    if (!chain.networkType) {
        throw new Error('Stacks config is invalid, networkType is missing');
    }

    let privateKey;
    if (options.mnemonic) {
        const mnemonic = options.mnemonic

        const wallet = await generateWallet({
            secretKey: mnemonic,
            password: '',
        });

        privateKey = wallet.accounts[0].stxPrivateKey;
    } else {
        privateKey = options.privateKey;
    }

    return {
        privateKey,
        stacksAddress: privateKeyToAddress(privateKey, chain.networkType),
        networkType: chain.networkType,
    };
}

async function createStacksWallet(chain) {
    if (!chain.networkType) {
        throw new Error('Stacks config is invalid, networkType is missing');
    }

    const mnemonic = generateSecretKey();
    const wallet = await generateWallet({
        secretKey: mnemonic,
        password: '',
    });

    const privateKey = wallet.accounts[0].stxPrivateKey;

    return {
        mnemonic,
        stacksAddress: privateKeyToAddress(privateKey, chain.networkType),
    };
}

module.exports = {
    getWallet,
    createStacksWallet,
};
