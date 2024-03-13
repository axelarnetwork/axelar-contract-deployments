const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require('../axelar-chains-config/info/mainnet.json');
const tokenManagerInfo = require('../axelar-chains-config/info/tokenManagers.json');
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const fs = require('fs');
const toml = require('toml');

const RPCs = toml.parse(fs.readFileSync('./axelar-chains-config/info/rpcs.toml', 'utf-8'));

// This is before the its was deployed on mainnet.
const startTimestamp = 1702800000;
const eventsLength = 2000;

async function getTokenManagers(name) {
    try {
        const chain = info.chains[name];
        if(chain.contracts.InterchainTokenService.skip) return;

        const rpc = RPCs.axelar_bridge_evm.find((chain) => chain.name.toLowerCase() === name.toLowerCase()).rpc_addr;
        const provider = getDefaultProvider(rpc);

        const  its = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, provider);
        
        const blockNumber = await provider.getBlockNumber();
        if(!tokenManagerInfo[name]) {
            tokenManagerInfo[name] = {start: blockNumber, end: blockNumber, tokenManagers: []};
        }

        const filter = its.filters.TokenManagerDeployed();
        while(blockNumber > tokenManagerInfo[name].end) {
            let end = blockNumber > tokenManagerInfo[name].end + eventsLength ? blockNumber : tokenManagerInfo[name].end + eventsLength;
            const events = await its.queryFilter(filter, tokenManagerInfo[name].end + 1, end);
            tokenManagerInfo[name].tokenManagers = tokenManagerInfo[name].tokenManagers.concat(events.map(event => event.args));
            tokenManagerInfo[name].end = end;
            fs.writeFileSync('./axelar-chains-config/info/tokenManagers.json', JSON.stringify(tokenManagerInfo, null, 2));
        }

        while(true) {
            const block = await provider.getBlock(tokenManagerInfo[name].start);
            console.log(name, block.timestamp);
            if(block.timestamp < startTimestamp || tokenManagerInfo[name].start < 0) break;
            const events = await its.queryFilter(filter, tokenManagerInfo[name].start - eventsLength, tokenManagerInfo[name].start-1);

            tokenManagerInfo[name].tokenManagers = tokens[name].tokenManagers.concat(events.map(event => event.args));
            tokenManagerInfo[name].start -= eventsLength;
            fs.writeFileSync('./axelar-chains-config/info/tokenManagers.json', JSON.stringify(tokenManagerInfo, null, 2))
        }
    } catch (e) {
        console.log(name);
        console.log(e);
    }
}
(async() => {   
    for(const name of Object.keys(info.chains)) {
        // add an await to run in sequence, which is slower.
        getTokenManagers(name);
    }
})();