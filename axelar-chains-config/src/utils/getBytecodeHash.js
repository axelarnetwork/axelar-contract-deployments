const { keccak256 } = require('@ethersproject/keccak256');
/**
 * Compute bytecode hash for a deployed contract or contract factory as it would appear on-chain.
 * Some chains don't use keccak256 for their state representation, which is taken into account by this function.
 * @param {Object} contractObject - An instance of the contract or a contract factory (ethers.js Contract or ContractFactory object)
 * @returns {Promise<string>} - The keccak256 hash of the contract bytecode
 */

async function getBytecodeHash(contractObject, chain = '', provider = null) {
    let bytecode;

    if (isString(contractObject)) {
        if (provider === null) {
            throw new Error('Provider must be provided for chain');
        }

        bytecode = await provider.getCode(contractObject);
    } else if (contractObject.address) {
        // Contract instance
        provider = contractObject.provider;
        bytecode = await provider.getCode(contractObject.address);
    } else if (contractObject.bytecode) {
        // Contract factory
        bytecode = contractObject.bytecode;
    } else {
        throw new Error('Invalid contract object. Expected ethers.js Contract or ContractFactory.');
    }

    if (chain.toLowerCase() === 'polygon-zkevm') {
        throw new Error('polygon-zkevm is not supported');
    }

    return keccak256(bytecode);
}

const isString = (arg) => {
    return typeof arg === 'string' && arg !== '';
};

module.exports = {
    getBytecodeHash,
};
