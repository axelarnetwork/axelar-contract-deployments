const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require('../axelar-chains-config/info/mainnet.json');
const tokenManagerInfo = require('../axelar-chains-config/info/tokenManagers.json');
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const fs = require('fs');
const toml = require('toml');

const RPCs = toml.parse(fs.readFileSync('./axelar-chains-config/info/rpcs.toml', 'utf-8'));

// This is before the its was deployed on mainnet.
// const startTimestamp = 1702800000;
let eventsLength = 2000;
const queryLimit = {
    ethereum: 500000,
    avalanche: 2048,
    fantom: 500000,
    polygon: 500000,
    moonbeam: 2000,
    binance: 10000,
    arbitrum: 500000,
    celo: 2000,
    kava: 10000,
    filecoin: 0,
    optimism: 10000,
    linea: 500000,
    base: 10000,
    mantle: 10000,
    blast: 10000,
    fraxtal: 50000,
    scroll: 500000,
};

async function getTokenManagers(name) {
    try {
        const chain = info.chains[name];
        if (chain.contracts.InterchainTokenService.skip) return;

        // if (name != 'mantle') { return; }

        eventsLength = queryLimit[name.toLowerCase()];
        console.log('processing... ', name);
        console.log(name, eventsLength);

        const rpc = RPCs.axelar_bridge_evm.find((chain) => chain.name.toLowerCase() === name.toLowerCase()).rpc_addr;
        const provider = getDefaultProvider(rpc);

        const its = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, provider);

        const blockNumber = await provider.getBlockNumber();

        if (!tokenManagerInfo[name]) {
            tokenManagerInfo[name] = { start: blockNumber, end: blockNumber, tokenManagers: [] };
        }

        const filter = its.filters.TokenManagerDeployed();
        console.log('current block number: ', blockNumber);

        while (blockNumber > tokenManagerInfo[name].end) {
            // let end = blockNumber > tokenManagerInfo[name].end + eventsLength ? blockNumber : tokenManagerInfo[name].end + eventsLength;
            const end = tokenManagerInfo[name].end + eventsLength;
            console.log(end);
            const events = await its.queryFilter(filter, tokenManagerInfo[name].end + 1, end);
            tokenManagerInfo[name].tokenManagers = tokenManagerInfo[name].tokenManagers.concat(events.map((event) => event.args));
            tokenManagerInfo[name].end = end;
            fs.writeFileSync('./axelar-chains-config/info/tokenManagers.json', JSON.stringify(tokenManagerInfo, null, 2));
        }
    } catch (e) {
        console.log(name);
        console.log(e);
    }
}

(async () => {
    for (const name of Object.keys(info.chains)) {
        // add an await to run in sequence, which is slower.
        getTokenManagers(name);
    }
})();
