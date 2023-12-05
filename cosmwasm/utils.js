'use strict';

const { readFileSync } = require('fs');
const { calculateFee, GasPrice } = require('@cosmjs/stargate');
const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');
const { getSaltFromKey } = require('../evm/utils');
const { normalizeBech32 } = require('@cosmjs/encoding');

const pascalToSnake = (str) => str.replace(/([A-Z])/g, (group) => `_${group.toLowerCase()}`).replace(/^_/, '');

const isValidCosmosAddress = (str) => {
    try {
        normalizeBech32(str);
    } catch (error) {
        return false;
    }

    return true;
};

const fromHex = (str) => {
    const start = str.startsWith('0x') ? 2 : 0;
    return new Uint8Array(Buffer.from(str.slice(start), 'hex'));
};

const uploadContract = async (client, wallet, config, options) => {
    const [account] = await wallet.getAccounts();
    const { artifactPath, contractName, instantiate2, salt, aarch64, chainNames } = options;

    const wasm = readFileSync(`${artifactPath}/${pascalToSnake(contractName)}${aarch64 ? '-aarch64' : ''}.wasm`);

    const {
        axelar: { gasPrice, gasLimit },
    } = config;
    const uploadFee = calculateFee(gasLimit, GasPrice.fromString(gasPrice));
    return client.upload(account.address, wasm, uploadFee).then((result) => {
        const address = instantiate2
            ? instantiate2Address(
                  fromHex(result.checksum),
                  account.address,
                  fromHex(getSaltFromKey(salt || contractName.concat(chainNames))),
                  'axelar',
              )
            : null;

        return { codeId: result.codeId, address };
    });
};

const instantiateContract = async (client, wallet, initMsg, config, { contractName, salt, instantiate2, chainNames }) => {
    const [account] = await wallet.getAccounts();
    const contractConfig = config.axelar.contracts[contractName];

    const {
        axelar: { gasPrice, gasLimit },
    } = config;
    const initFee = calculateFee(gasLimit, GasPrice.fromString(gasPrice));

    return (
        instantiate2
            ? client.instantiate2(
                  account.address,
                  contractConfig.codeId,
                  fromHex(getSaltFromKey(salt || contractName.concat(chainNames))),
                  initMsg,
                  contractName,
                  initFee,
              )
            : client.instantiate(account.address, contractConfig.codeId, initMsg, contractName, initFee)
    ).then(({ contractAddress }) => contractAddress);
};

module.exports = {
    uploadContract,
    instantiateContract,
    isValidCosmosAddress,
};
