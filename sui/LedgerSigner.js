'use strict';

const { Keypair, toSerializedSignature } = require('@mysten/sui/cryptography');
const { Signer } = require('@mysten/sui/cryptography');
const { Secp256k1Keypair } = require('@mysten/sui/keypairs/secp256k1');
const { Secp256k1PublicKey } = require('@mysten/sui/keypairs/secp256k1');
const TransportNodeHid = require('@ledgerhq/hw-transport-node-hid').default;
const Sui = require('@mysten/ledgerjs-hw-app-sui').default;
const { toB64 } = require('@mysten/sui/utils');
const { publicKeyFromRawBytes } = require('@mysten/sui/verify');


class LedgerSigner extends Signer {
    constructor(path) {
        super();
        this.path = "44'/784'/0'/0'/0'";
    }

    async getPublicKey() {
        const sui = await this.getSuiTransport();
        return await sui.getPublicKey(this.path);
    }

    async toSuiAddress() {
        const sui = await this.getSuiTransport();
        return `0x${(await sui.getPublicKey(this.path)).address.toString('hex')}`;
    }

    async signTransaction(bytes){
        const sui = await this.getSuiTransport();
        const publicKey = publicKeyFromRawBytes('ED25519', (await sui.getPublicKey(this.path)).publicKey);
        return {signature: toSerializedSignature({...(await sui.signTransaction(this.path, bytes)), signatureScheme: 'ED25519', publicKey })};
    };

    async getSuiTransport(){
        return new Sui(await TransportNodeHid.create());
    };
}

module.exports = {
    LedgerSigner,
};
