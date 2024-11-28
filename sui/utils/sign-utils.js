'use strict';

const { verifyTransactionSignature } = require('@mysten/sui/verify');
const { decodeSuiPrivateKey } = require('@mysten/sui/cryptography');
const { Ed25519Keypair, Ed25519PublicKey } = require('@mysten/sui/keypairs/ed25519');
const { MultiSigPublicKey } = require('@mysten/sui/multisig');
const { Secp256k1Keypair, Secp256k1PublicKey } = require('@mysten/sui/keypairs/secp256k1');
const { Secp256r1Keypair, Secp256r1PublicKey } = require('@mysten/sui/keypairs/secp256r1');
const { SuiClient, getFullnodeUrl } = require('@mysten/sui/client');
const { fromB64, fromHEX } = require('@mysten/bcs');
const { execute } = require('@axelar-network/axelar-cgp-sui');
const { printInfo } = require('../../common/utils');
const { ethers } = require('hardhat');
const { LedgerSigner } = require('../LedgerSigner');
const {
    utils: { hexlify },
} = ethers;

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

    const url = chain.rpc || getFullnodeUrl(chain.networkType);
    const client = new SuiClient({ url });

    if(options.privateKey === 'ledger') {
        keypair = new LedgerSigner();
        return [keypair, client]
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

        case 'hex': {
            const privKey = Buffer.from(options.privateKey, 'hex');
            keypair = scheme.fromSecretKey(privKey);
            break;
        }

        default: {
            throw new Error(`Unsupported key type: ${options.privateKeyType}`);
        }
    }

    return [keypair, client];
}

async function printWalletInfo(wallet, client, chain, options) {
    const owner =
        wallet instanceof Ed25519Keypair || wallet instanceof Secp256k1Keypair || wallet instanceof Secp256r1Keypair
            ? wallet.toSuiAddress()
            : wallet;
    printInfo('Wallet address', owner);

    const coins = await client.getBalance({ owner });
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

async function broadcast(client, keypair, tx, actionName) {
    const receipt = await client.signAndExecuteTransaction({
        transaction: tx,
        signer: keypair,
        options: {
            showEffects: true,
            showObjectChanges: true,
            showContent: true,
        },
    });

    printInfo(actionName || 'Tx', receipt.digest);

    return receipt;
}

async function broadcastFromTxBuilder(txBuilder, keypair, actionName, suiResponseOptions) {
    const receipt = await txBuilder.signAndExecute(keypair, suiResponseOptions);

    printInfo(actionName || 'Tx', receipt.digest);

    return receipt;
}

const broadcastExecuteApprovedMessage = async (client, keypair, discoveryInfo, gatewayInfo, messageInfo, actionName) => {
    const receipt = await execute(client, keypair, discoveryInfo, gatewayInfo, messageInfo);

    printInfo(actionName || 'Tx', receipt.digest);

    return receipt;
};

async function broadcastSignature(client, txBytes, signature, actionName) {
    const receipt = await client.executeTransactionBlock({
        transactionBlock: txBytes,
        signature,
        options: {
            showEffects: true,
            showObjectChanges: true,
            showEvents: true,
        },
    });

    if (actionName) {
        printInfo(actionName, receipt.digest);
    }

    return receipt;
}

async function signTransactionBlockBytes(keypair, client, txBytes, options) {
    const serializedSignature = (await keypair.signTransaction(txBytes)).signature;
    let publicKey;

    try {
        publicKey = await verifyTransactionSignature(txBytes, serializedSignature);
    } catch {
        throw new Error(`Cannot verify tx signature`);
    }

    if (publicKey.toSuiAddress() !== keypair.toSuiAddress()) {
        throw new Error(`Verification failed for address ${keypair.toSuiAddress()}`);
    }

    if (!options.offline) {
        const txResult = await broadcastSignature(client, txBytes, serializedSignature);
        printInfo('Transaction result', JSON.stringify(txResult));
    } else {
        const hexPublicKey = hexlify(publicKey.toRawBytes());
        return {
            signature: serializedSignature,
            publicKey: hexPublicKey,
        };
    }
}

async function signTransaction(chain, txDetails, options) {
    const { txBlock, buildOptions = {} } = txDetails;

    const [keypair, client] = getWallet(chain, options);
    txBlock.setSenderIfNotSet(keypair.toSuiAddress());
    const txBytes = await txBlock.build(buildOptions);

    const result = await signTransactionBlockBytes(keypair, client, txBytes, options);
    result.txBytes = txBytes;

    return result;
}

async function getWrappedPublicKey(bech64PublicKey, schemeType) {
    const uint8PubKey = fromB64(bech64PublicKey).slice(1);

    switch (schemeType) {
        case 'ed25519': {
            return new Ed25519PublicKey(uint8PubKey);
        }

        case 'secp256k1': {
            return new Secp256k1PublicKey(uint8PubKey);
        }

        case 'secp256r1': {
            return new Secp256r1PublicKey(uint8PubKey);
        }

        default: {
            throw new Error(`Unsupported signature scheme: ${schemeType}`);
        }
    }
}

async function getMultisig(config, multisigKey) {
    let multiSigPublicKey;

    if (multisigKey) {
        multiSigPublicKey = new MultiSigPublicKey(fromHEX(multisigKey));
    } else {
        const signers = config.multisig?.signers;

        if (!signers || signers.length === 0) {
            throw new Error('Signers not provided in configuration');
        }

        const publicKeys = [];

        for (const signer of signers) {
            if (!signer?.publicKey) {
                throw new Error('PublicKey not found');
            }

            if (!signer?.schemeType) {
                throw new Error('SchemeType not found');
            }

            if (!signer?.weight) {
                throw new Error('Weight not found');
            }

            publicKeys.push({
                publicKey: await getWrappedPublicKey(signer.publicKey, signer.schemeType),
                weight: signer.weight,
            });
        }

        multiSigPublicKey = MultiSigPublicKey.fromPublicKeys({
            threshold: config.multisig?.threshold,
            publicKeys,
        });
    }

    printInfo('Multisig Wallet Address', multiSigPublicKey.toSuiAddress());

    return multiSigPublicKey;
}

module.exports = {
    getWallet,
    printWalletInfo,
    generateKeypair,
    getRawPrivateKey,
    broadcast,
    broadcastSignature,
    signTransaction,
    getMultisig,
    getWrappedPublicKey,
    signTransactionBlockBytes,
    broadcastFromTxBuilder,
    broadcastExecuteApprovedMessage,
};
