'use strict';

const { readFileSync } = require('fs');
const { calculateFee, GasPrice } = require('@cosmjs/stargate');
const { instantiate2Address } = require('@cosmjs/cosmwasm-stargate');
const { getSaltFromKey} = require('../evm/utils');

const pascalToSnake = (str) => {
    return str.replace(/([A-Z])/g, (group) => `_${group.toLowerCase()}`).replace(/^_/, '');
};

const uploadContract = async (config, options, wallet, client) => {
    const [account] = await wallet.getAccounts();

    const wasm = readFileSync(`${options.artifactPath}/${pascalToSnake(options.contractName)}${options.aarch64 ? '-aarch64' : ''}.wasm`);

    const gasPrice = GasPrice.fromString(`0.00005u${config.axelar.tokenSymbol.toLowerCase()}`);
    const uploadFee = calculateFee(5000000, gasPrice);
    const result = await client.upload(account.address, wasm, uploadFee);
    var address = null;
    if (!!options.instantiate2) {
        const salt = getSaltFromKey(options.salt || options.contractName);

        const checksum = Uint8Array.from(Buffer.from(result.checksum, 'hex'));
        address = instantiate2Address(checksum, account.address, new Uint8Array(Buffer.from(salt.slice(2),'hex')), "axelar")
    }

    return {codeId: result.codeId, address: address}
};

const instantiateContract = async (config, options, contractName, initMsg, wallet, client) => {
    const [account] = await wallet.getAccounts();
    const contractConfig = config.axelar.contracts[contractName];

    const gasPrice = GasPrice.fromString(`0.00005u${config.axelar.tokenSymbol.toLowerCase()}`);
    const initFee = calculateFee(500000, gasPrice);

    var result;
    if (!!options.instantiate2) {
        const salt = getSaltFromKey(options.salt || options.contractName);
        result = await client.instantiate2(account.address, contractConfig.codeId, new Uint8Array(Buffer.from(salt.slice(2), 'hex')), initMsg, contractName, initFee);
    }
    else {
        result = await client.instantiate(account.address, contractConfig.codeId, initMsg, contractName, initFee);
    }

    return result.contractAddress;
};

module.exports = {
    pascalToSnake,
    uploadContract,
    instantiateContract,
};
