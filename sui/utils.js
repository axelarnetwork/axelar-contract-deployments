'use strict';

const { ethers } = require('hardhat');
const {
    BigNumber,
    utils: { arrayify },
} = ethers;
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

const getAmplifierSigners = async (config, chain) => {
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    const workerSet = await client.queryContractSmart(config.axelar.contracts.MultisigProver[chain].address, 'get_worker_set');
    const signers = Object.values(workerSet.signers);

    const weightedSigners = signers
        .map((signer) => ({
            pubkey: arrayify(`0x${signer.pub_key.ecdsa}`),
            weight: Number(signer.weight),
        }))
        .sort((a, b) => a.pubkey.localeCompare(b.pubkey));

    return {
        signers: weightedSigners,
        threshold: Number(workerSet.threshold),
        nonce: ethers.utils.hexZeroPad(BigNumber.from(workerSet.created_at).toHexString(), 32),
    };
};

module.exports = {
    getAmplifierSigners,
};
