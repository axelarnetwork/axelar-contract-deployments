'use strict';

const { getContractPath } = require('../evm/utils.js');
const { linkBytecode } = require('solc/linker');
const { printInfo } = require('../common/utils');

const HTS_DEPENDENT_CONTRACTS = ['InterchainTokenDeployer', 'TokenManager', 'InterchainTokenService', 'InterchainTokenFactory'];

const HTS_LIBRARY_NAME = 'HTS';
const HTS_LIBRARY_SOURCE = `contracts/hedera/HTS.sol:${HTS_LIBRARY_NAME}`;

function getContractJSONWithHTS(htsLibAddress) {
    return (contractName, artifactPath) => {
        let contractPath;

        if (artifactPath) {
            contractPath = artifactPath.endsWith('.json') ? artifactPath : artifactPath + contractName + '.sol/' + contractName + '.json';
        } else {
            contractPath = getContractPath(contractName);
        }

        try {
            const contractJson = require(contractPath);

            // Link contracts that depend on HTS library
            if (HTS_DEPENDENT_CONTRACTS.includes(contractName)) {
                if (!htsLibAddress) {
                    throw new Error('HTS library address is required.');
                }

                printInfo(`Linking ${contractName} bytecode with HTS library.`);
                contractJson.bytecode = linkBytecode(contractJson.bytecode, {
                    [HTS_LIBRARY_NAME]: htsLibAddress,
                    [HTS_LIBRARY_SOURCE]: htsLibAddress,
                });
            }

            return contractJson;
        } catch (err) {
            throw new Error(`Failed to load contract JSON for ${contractName} at path ${contractPath} with error: ${err}`);
        }
    };
}

module.exports = {
    getContractJSONWithHTS,
    HTS_LIBRARY_NAME,
};
