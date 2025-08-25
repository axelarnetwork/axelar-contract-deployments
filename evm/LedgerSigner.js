'use strict';

const { ethers } = require('hardhat');
const {
    BigNumber,
    Signer,
    VoidSigner,
    utils: { serializeTransaction },
} = ethers;
const TransportNodeHid = require('@ledgerhq/hw-transport-node-hid').default;
const Eth = require('@ledgerhq/hw-app-eth').default;

const { printInfo } = require('./utils');

class LedgerSigner extends Signer {
    constructor(provider, path = "m/44'/60'/0'/0/0") {
        super();
        this.path = path;
        this.provider = provider;
    }

    async connect(provider = null) {
        if (provider) {
            this.provider = provider;
        }

        this.transport = await TransportNodeHid.open();
        this.eth = new Eth(this.transport);
    }

    async getAddress() {
        if (!this.eth) await this.connect();
        const result = await this.eth.getAddress(this.path);
        return result.address;
    }

    async signMessage(message) {
        if (!this.eth) await this.connect();

        if (typeof message === 'string') {
            message = ethers.utils.toUtf8Bytes(message);
        }

        const messageHex = ethers.utils.hexlify(message).substring(2);

        const sig = await this.eth.signPersonalMessage(this.path, messageHex);

        return ethers.utils.joinSignature(await this._fixSignature(sig, 2, 1));
    }

    async signTransaction(tx) {
        if (!this.eth) await this.connect();

        delete tx.from;

        tx = await ethers.utils.resolveProperties(tx);

        console.log('Unsigned tx', tx);

        const rawTx = serializeTransaction(tx).substring(2);

        const sig = await this._fixSignature(await this.eth.signTransaction(this.path, rawTx, null), tx.type, tx.chainId);

        const signedTx = serializeTransaction(tx, sig);

        printInfo('Signed Tx', signedTx);

        return signedTx;
    }

    async populateTransaction(tx) {
        if (!this.eth) await this.connect();

        return new VoidSigner(await this.getAddress(), this.provider).populateTransaction(tx);
    }

    async _fixSignature(signature, type, chainId) {
        let v = BigNumber.from('0x' + signature.v).toNumber();

        if (type === 2) {
            // EIP-1559 transaction. Nothing to do.
            // v is already returned as 0 or 1 by Ledger for Type 2 txs
        } else {
            // Undefined or Legacy Type 0 transaction. Ledger computes EIP-155 sig.v computation incorrectly in this case
            // v in {0,1} + 2 * chainId + 35
            // Ledger gives this value mod 256
            // So from that, compute whether v is 0 or 1 and then add to 2 * chainId + 35 without doing a mod
            v = 2 * chainId + 35 + ((v + 256 * 100000000000 - (2 * chainId + 35)) % 256);
        }

        return {
            r: '0x' + signature.r,
            s: '0x' + signature.s,
            v,
        };
    }

    disconnect() {
        if (this.transport) {
            this.transport.close();
        }
    }
}

module.exports = {
    LedgerSigner,
};
