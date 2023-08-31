require('@nomicfoundation/hardhat-toolbox');

const env = process.env.NETWORK || 'testnet';
const { importNetworks, readJSON } = require('./evm/utils.js');
const chains = require(`${__dirname}/info/${env}.json`);
const keys = readJSON(`${__dirname}/info/keys.json`);
const { networks, etherscan } = importNetworks(chains, keys);

/**
 * @type import('hardhat/config').HardhatUserConfig
 */
module.exports = {
    solidity: {
        version: '0.8.19',
        settings: {
            evmVersion: process.env.EVM_VERSION || 'london',
            optimizer: {
                enabled: true,
                runs: 1000000,
                details: {
                    peephole: process.env.COVERAGE === undefined,
                    inliner: process.env.COVERAGE === undefined,
                    jumpdestRemover: true,
                    orderLiterals: true,
                    deduplicate: true,
                    cse: process.env.COVERAGE === undefined,
                    constantOptimizer: true,
                    yul: true,
                    yulDetails: {
                        stackAllocation: true,
                    },
                },
            },
        },
    },
    defaultNetwork: 'hardhat',
    networks,
    etherscan,
    mocha: {
        timeout: 1000000,
    },
    gasReporter: {
        enabled: process.env.REPORT_GAS !== '',
    },
};
