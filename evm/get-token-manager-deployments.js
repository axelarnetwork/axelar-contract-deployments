require('dotenv').config();
const env = process.env.ENV;

const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const fs = require('fs');
const { printInfo, printError } = require('../common/utils');

// const RPCs = require(`../axelar-chains-config/rpcs/${env}.json`);

// This is before the its was deployed on mainnet.
// const startTimestamp = 1702800000;
// This is after the upgrade.
const endTimestamp = 1710329513;
const queryLimit = {
    ethereum: 500000,
    'eth-sepolia': 1000,
    'ethereum-sepolia': 1000,
    'core-ethereum': 1000,
    avalanche: 2048,
    'core-avalanche': 10000,
    fantom: 500000,
    polygon: 500000,
    'polygon-sepolia': 500000,
    moonbeam: 2000,
    binance: 10000,
    arbitrum: 500000,
    'arbitrum-sepolia': 10000,
    celo: 50000,
    kava: 10000,
    filecoin: 2880,
    optimism: 10000,
    'optimism-sepolia': 10000,
    linea: 500000,
    'linea-sepolia': 500000,
    base: 10000,
    'base-sepolia': 10000,
    mantle: 10000,
    'mantle-sepolia': 10000,
    blast: 10000,
    'blast-sepolia': 10000,
    fraxtal: 50000,
    scroll: 10000,
    flow: 10000,
    immutable: 5000,
};

async function getTokenManagersFromBlock(its, filter, startBlockNumber, eventsLength, max) {
    const end = Math.min(startBlockNumber + eventsLength, max);
    for (let i = 0; i < 30; i++) {
        try {
            const events = await its.queryFilter(filter, startBlockNumber, end);
            return events;
        } catch (e) {}
    }
    return false;
}

async function getTokenManagers(name, tokenManagerInfo) {
    try {
        const chain = info.chains[name];
        if (!chain.contracts.InterchainTokenService || chain.contracts.InterchainTokenService.skip) return false;
        printInfo(`ITS at ${name} is at`, chain.contracts.InterchainTokenService.address);

        const eventsLength = queryLimit[name.toLowerCase()] || 2048;
        printInfo('processing... ', name);

        // const rpc = RPCs[name];
        const rpc = chain.rpc;
        printInfo(name, rpc);
        if (!rpc) {
            printError(`No RPC for ${name}`);
            return false;
        }
        const provider = getDefaultProvider(rpc);
        const its = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, provider);
        const max = await provider.getBlockNumber();

        if (!tokenManagerInfo[name]) {
            tokenManagerInfo[name] = { start: 0, end: 0, max: max, alreadyProcessedPercentage: 0, tokenManagers: [] };
        }

        const filter = its.filters.TokenManagerDeployed();
        printInfo(`${name} current block number: ${max}`);

        //if ((await provider.getBlock(tokenManagerInfo[name].end)).timestamp >= endTimestamp) return;

        /*while (max - min > 1) {
            const mid = Math.floor((min + max) / 2);
            const timestamp = (await provider.getBlock(mid)).timestamp;
            if (timestamp > endTimestamp) {
                max = mid;
            } else {
                min = mid;
            }
        }
        printInfo('Target Block number', min);*/

        const batchSize = 30;
        while (tokenManagerInfo[name].end < max) {
            let eventsPromises = [];
            for (let i = 0; i < batchSize; i++) {
                const newEventsPromises = getTokenManagersFromBlock(
                    its,
                    filter,
                    tokenManagerInfo[name].end + 1 + i * eventsLength,
                    eventsLength,
                    max,
                );
                eventsPromises = eventsPromises.concat(newEventsPromises);
            }

            // Wait for all promises to resolve
            const tokenManagerData = await Promise.all(eventsPromises);
            if (tokenManagerData.includes(false)) {
                printError(`Failed to get token managers for ${name} after 30 tries`);
                return false;
            }
            tokenManagerInfo[name].tokenManagers = tokenManagerInfo[name].tokenManagers.concat(
                tokenManagerData
                    .flat()
                    .map((event) => event.args)
                    .map((args) => {
                        return {
                            tokenId: args[0],
                            tokenManagerAddress: args[1],
                            tokenManagerType: args[2],
                            deployParams: args[3],
                        };
                    }),
            );
            tokenManagerInfo[name].end += batchSize * eventsLength;
            tokenManagerInfo[name].alreadyProcessedPercentage = ((tokenManagerInfo[name].end / tokenManagerInfo[name].max) * 100).toFixed(
                2,
            );
        }
        return true;
    } catch (e) {
        printError(`Error getting token managers for ${name}: ${e.message}`);
        return false;
    }
}

(async () => {
    tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);

    const promises = [];
    for (const name of Object.keys(info.chains)) {
        promises.push(getTokenManagers(name, tokenManagerInfo));
    }
    // write to file every second
    setInterval(() => {
        fs.writeFileSync(`./axelar-chains-config/info/tokenManagers-${env}.json`, JSON.stringify(tokenManagerInfo, null, 2));
    }, 1000);

    await Promise.all(promises).then(() => {
        fs.writeFileSync(`./axelar-chains-config/info/tokenManagers-${env}.json`, JSON.stringify(tokenManagerInfo, null, 2));
        process.exit(0);
    });
})();
