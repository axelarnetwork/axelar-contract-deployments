// Example hardhat config to import custom networks
require('@nomicfoundation/hardhat-toolbox');

const env = process.env.ENV || 'testnet';
const { importNetworks, readJSON } = require('@axelar-network/axelar-contract-deployments');
const chains = require(`@axelar-network/axelar-contract-deployments/axelar-chains-config/info/${env}.json`);
const keys = readJSON(`${__dirname}/keys.json`);
const { networks, etherscan } = importNetworks(chains, keys);

module.exports = {
    solidity: {
        version: '0.8.9',
        settings: {},
    },
    defaultNetwork: 'hardhat',
    networks: networks,
    etherscan: etherscan,
    mocha: {
        timeout: 1000000, // Keep a higher timeout since tests on live networks can take much longer
    },
    gasReporter: {
        enabled: process.env.REPORT_GAS !== '',
    },
};
