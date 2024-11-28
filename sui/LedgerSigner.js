'use strict';

const { Signer } = require('@mysten/sui/cryptography');
const Transport = require('@ledgerhq/hw-transport').default;
const Sui = require('@mysten/ledgerjs-hw-app-sui').default;
const { toB64 } = require('@mysten/sui/utils');


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
        return toB64(await sui.getPublicKey(this.path).address);
    }

    async signTransaction(bytes){
        const sui = await this.getSuiTransport();
        return await sui.signTransaction(this.path, bytes);
    };

    async sign(bytes){
        const sui = await this.getSuiTransport();
        return (await sui.signTransaction(this.path, bytes)).signature;
    };

    async getSuiTransport(){
        return new Sui(await Transport.create());
    };

    getKeyScheme(){
        return 'Secp256k1';
    };
}

module.exports = {
    LedgerSigner,
};
