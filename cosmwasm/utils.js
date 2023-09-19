'use strict';

const { readFileSync } = require('fs');
const { calculateFee, GasPrice } = require('@cosmjs/stargate');

const pascalToSnake = (str) => {
    return str.replace(/([A-Z])/g, (group) => `_${group.toLowerCase()}`).replace(/^_/, '');
};

const uploadContract = async (config, options, wallet, client) => {
    const [account] = await wallet.getAccounts();

    const wasm = readFileSync(`${options.artifactPath}/${pascalToSnake(options.contractName)}${options.aarch64 ? '-aarch64' : ''}.wasm`);

    const gasPrice = GasPrice.fromString(`0.00005u${config.axelar.tokenSymbol.toLowerCase()}`);
    const uploadFee = calculateFee(5000000, gasPrice);

    const result = await client.upload(account.address, wasm, uploadFee);

    return result.codeId;
};

const instantiateContract = async (config, contractName, initMsg, wallet, client) => {
    const [account] = await wallet.getAccounts();
    const contractConfig = config.axelar.contracts[contractName];

    const gasPrice = GasPrice.fromString(`0.00005u${config.axelar.tokenSymbol.toLowerCase()}`);
    const initFee = calculateFee(500000, gasPrice);

    const result = await client.instantiate(account.address, contractConfig.codeID, initMsg, contractName, initFee);

    return result.contractAddress;
};

module.exports = {
    pascalToSnake,
    uploadContract,
    instantiateContract,
};
