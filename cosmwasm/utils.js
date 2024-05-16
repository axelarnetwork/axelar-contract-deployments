'use strict';

const { readFileSync } = require('fs');
const { calculateFee, GasPrice } = require('@cosmjs/stargate');
const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');
const { getSaltFromKey } = require('../evm/utils');
const { normalizeBech32 } = require('@cosmjs/encoding');

const governanceAddress = 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';

const pascalToSnake = (str) => str.replace(/([A-Z])/g, (group) => `_${group.toLowerCase()}`).replace(/^_/, '');

const isValidCosmosAddress = (str) => {
    try {
        normalizeBech32(str);

        return true;
    } catch (error) {
        return false;
    }
};

const fromHex = (str) => new Uint8Array(Buffer.from(str.replace('0x', ''), 'hex'));

const uploadContract = async (client, wallet, config, options) => {
    const { artifactPath, contractName, instantiate2, salt, aarch64, chainNames } = options;
    return wallet
        .getAccounts()
        .then(([account]) => {
            const wasm = readFileSync(`${artifactPath}/${pascalToSnake(contractName)}${aarch64 ? '-aarch64' : ''}.wasm`);
            const {
                axelar: { gasPrice, gasLimit },
            } = config;

            console.log('gasLimit:', gasLimit);
            console.log('gasPrice:', gasPrice);

            console.log('account:', account);

            const uploadFee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));

            console.log('uploadFee:', uploadFee);

            return client.upload(account.address, wasm, uploadFee).then(({ checksum, codeId }) => ({ checksum, codeId, account }));
        })
        .then(({ account, checksum, codeId }) => {
            const address = instantiate2
                ? instantiate2Address(
                      fromHex(checksum),
                      account.address,
                      fromHex(getSaltFromKey(salt || contractName.concat(chainNames))),
                      'axelar',
                  )
                : null;

            return { codeId, address };
        });
};

const instantiateContract = (client, wallet, initMsg, config, { contractName, salt, instantiate2, chainNames, admin }) => {
    return wallet
        .getAccounts()
        .then(([account]) => {
            const contractConfig = config.axelar.contracts[contractName];

            const {
                axelar: { gasPrice, gasLimit },
            } = config;
            const initFee = gasLimit === 'auto' ? 'auto' : calculateFee(gasLimit, GasPrice.fromString(gasPrice));

            return instantiate2
                ? client.instantiate2(
                      account.address,
                      contractConfig.codeId,
                      fromHex(getSaltFromKey(salt || contractName.concat(chainNames))),
                      initMsg,
                      contractName,
                      initFee,
                      { admin },
                  )
                : client.instantiate(account.address, contractConfig.codeId, initMsg, contractName, initFee, {
                      admin,
                  });
        })
        .then(({ contractAddress }) => contractAddress);
};

module.exports = {
    governanceAddress,
    uploadContract,
    instantiateContract,
    isValidCosmosAddress,
};
