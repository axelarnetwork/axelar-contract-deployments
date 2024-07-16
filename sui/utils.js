'use strict';

const { ethers } = require('hardhat');
const { loadConfig } = require('../evm/utils');
const {
    BigNumber,
    utils: { arrayify, hexlify },
} = ethers;
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { updateMoveToml, copyMovePackage, TxBuilder } = require('@axelar-network/axelar-cgp-sui');

const getAmplifierSigners = async (config, chain) => {
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    const workerSet = await client.queryContractSmart(config.axelar.contracts.MultisigProver[chain].address, 'current_verifier_set');
    const signers = Object.values(workerSet.signers);

    const weightedSigners = signers
        .map((signer) => ({
            pubkey: arrayify(`0x${signer.pub_key.ecdsa}`),
            weight: Number(signer.weight),
        }))
        .sort((a, b) => hexlify(a.pubkey).localeCompare(hexlify(b.pubkey)));

    return {
        signers: weightedSigners,
        threshold: Number(workerSet.threshold),
        nonce: ethers.utils.hexZeroPad(BigNumber.from(workerSet.created_at).toHexString(), 32),
    };
};

const loadSuiConfig = (env) => {
    const config = loadConfig(env);
    const suiEnv = env === 'local' ? 'localnet' : env;

    if (!config.sui) {
        config.sui = {
            networkType: suiEnv,
            name: 'Sui',
            contracts: {
                axelar_gateway: {},
            },
        };
    }

    return config;
};

const deployPackage = async (packageName, client, keypair) => {
    const compileDir = `${__dirname}/move`;
    copyMovePackage(packageName, null, compileDir);
    const builder = new TxBuilder(client);
    await builder.publishPackageAndTransferCap(packageName, keypair.toSuiAddress(), compileDir);
    const publishTxn = await builder.signAndExecute(keypair);

    const packageId = (publishTxn.objectChanges?.find((a) => a.type === 'published') ?? []).packageId;

    updateMoveToml(packageName, packageId, compileDir);
    return { packageId, publishTxn };
}

module.exports = {
    getAmplifierSigners,
    loadSuiConfig,
    deployPackage,
};
