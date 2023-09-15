/**
 * Imports custom networks into hardhat config format.
 * Check out the example hardhat config for usage `.example.hardhat.config.js`.
 *
 * @param {Object[]} chains - Array of chain objects following the format in info/mainnet.json
 * @param {Object} keys - Object containing keys for contract verification and accounts
 * @returns {Object} - Object containing networks and etherscan config
 */
const importNetworks = (chains, keys) => {
    const networks = {
        hardhat: {
            chainId: 31337, // default hardhat network chain id
            id: 'hardhat',
            confirmations: 1,
        },
    };

    const etherscan = {
        apiKey: {},
        customChains: [],
    };

    if (!chains.chains) {
        // Use new format
        delete chains.chains;
        chains = {
            chains,
        };
    }

    // Add custom networks
    Object.entries(chains.chains).forEach(([chainName, chain]) => {
        const name = chainName.toLowerCase();
        networks[name] = {
            ...chain,
            url: chain.rpc,
            blockGasLimit: chain.gasOptions?.gasLimit,
        };

        if (keys) {
            networks[name].accounts = keys.accounts || keys.chains[name]?.accounts;
        }

        // Add contract verification keys
        if (chain.explorer?.api) {
            if (keys) {
                etherscan.apiKey[name] = keys.chains[name]?.api;
            }

            etherscan.customChains.push({
                network: name,
                chainId: chain.chainId,
                urls: {
                    apiURL: chain.explorer.api,
                    browserURL: chain.explorer.url,
                },
            });
        }
    });

    return { networks, etherscan };
};

module.exports = {
    importNetworks,
};
