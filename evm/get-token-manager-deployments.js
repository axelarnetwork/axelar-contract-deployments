require('dotenv').config();
const env = process.env.ENV;

const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const fs = require('fs');
const toml = require('toml');
const { printInfo } = require('../common');

// const RPCs = require(`../axelar-chains-config/rpcs/${env}.json`);

// This is before the its was deployed on mainnet.
// const startTimestamp = 1702800000;
// This is after the upgrade.
const endTimestamp = 1710329513;
const queryLimit = {
    ethereum: 500000,
    "eth-sepolia": 1000,
    "ethereum-sepolia": 1000,
    "core-ethereum": 1000,
    avalanche: 2048,
    "core-avalanche": 10000,
    fantom: 500000,
    polygon: 500000,
    "polygon-sepolia": 500000,
    moonbeam: 2000,
    binance: 10000,
    arbitrum: 500000,
    "arbitrum-sepolia": 10000,
    celo: 50000,
    kava: 10000,
    filecoin: 2880,
    optimism: 10000,
    "optimism-sepolia": 10000,
    linea: 500000,
    "linea-sepolia": 500000,
    base: 10000,
    "base-sepolia": 10000,
    mantle: 10000,
    "mantle-sepolia": 10000,
    blast: 10000,
    "blast-sepolia": 10000,
    fraxtal: 50000,
    scroll: 10000,
    flow: 10000,
    immutable: 5000,
};

async function getTokenManagers(name) {
    try {
        const chain = info.chains[name];
        if (!chain.contracts.InterchainTokenService || chain.contracts.InterchainTokenService.skip) return false;
        printInfo(`ITS at ${name} is at`, chain.contracts.InterchainTokenService.address );

        const eventsLength = queryLimit[name.toLowerCase()] || 2048;
        console.log('processing... ', name);

        // const rpc = RPCs[name];
        const rpc = chain.rpc;
        console.log(name, rpc);
        if(!rpc) return false;
        const provider = getDefaultProvider(rpc);

        const its = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, provider);
        
        const blockNumber = await provider.getBlockNumber();

        if (!tokenManagerInfo[name]) {
            tokenManagerInfo[name] = { start: 1, end: 1, tokenManagers: [] };
        }

        const filter = its.filters.TokenManagerDeployed();
        console.log(name, 'current block number: ', blockNumber);

        let min = tokenManagerInfo[name].end;
        let max = blockNumber;

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
        min = blockNumber;
        let tries = 0;
        while (tokenManagerInfo[name].end < min) {
            try {
                const end = min < tokenManagerInfo[name].end + eventsLength ? min : tokenManagerInfo[name].end + eventsLength;
                console.log(name, end, min, eventsLength);
                const events = await its.queryFilter(filter, tokenManagerInfo[name].end + 1, end);
                tokenManagerInfo[name].tokenManagers = tokenManagerInfo[name].tokenManagers.concat(events.map((event) => event.args));
                tokenManagerInfo[name].end = end;
                fs.writeFileSync(`./axelar-chains-config/info/tokenManagers-${env}.json`, JSON.stringify(tokenManagerInfo, null, 2));
                tries = 0;
            } catch (e) {
                tries++;
                if (tries >= 30) {
                    console.log(e);
                    return false;
                }
            }
        }
        return true;
    } catch (e) {
        console.log(name);
        console.log(e);
        return false;
    }
}

(async () => {
    let results = {};
    for (const name of Object.keys(info.chains)) {
        results[name] = 0;
        // add an await to run in sequence, which is slower.
        getTokenManagers(name).then((result) => {
            results[name] = result;
            console.log(name);
            console.log(results);
        });
    }
})();
