const fs = require('fs');
const path = require('path');

/**
 * Get the chain config for a given environment (Currently only supports 'mainnet', 'testnet' and 'stagenet'). The returned value will be an array of chains instead of key-value pairs.
 * @param {*} env - The environment to get the chain config for (e.g. 'mainnet', 'testnet', 'stagenet')
 * @returns {Array} - An array of chain configs
 */
function getChainArray(env) {
    const _path = path.join(__dirname, '..', '..', 'info');
    const files = fs.readdirSync(_path);
    const file = `${env}.json`;

    if (!files.includes(file)) {
        throw new Error(`Env ${env} not found in 'info' directory`);
    }

    const data = fs.readFileSync(path.join(_path, file));
    const json = JSON.parse(data);

    const chains = [];

    for (const chain in json.chains) {
        chains.push(json.chains[chain]);
    }

    return chains;
}

module.exports = {
    getChainArray,
};
