const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require('../axelar-chains-config/info/mainnet.json');
const tokenManagerInfo = require('../axelar-chains-config/info/tokenManagers.json');
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const fs = require('fs');
const toml = require('toml');

const RPCs = toml.parse(fs.readFileSync('./axelar-chains-config/info/rpcs.toml', 'utf-8'));

async function getTokens(name) {
    if(!tokenManagerInfo[name]) return;
    const tokenManagers = tokenManagerInfo[name].tokenManagers;
    const chain = info.chains[name];
    if(chain.contracts.InterchainTokenService.skip) return;

    const rpc = RPCs.axelar_bridge_evm.find((chain) => chain.name.toLowerCase() === name.toLowerCase()).rpc_addr;
    const provider = getDefaultProvider(rpc);

    for(const tokenData of tokenManagers) {
        const tokenManager = new Contract(tokenData[1], ITokenManager.abi, provider);
        tokenData.tokenAddress = await tokenManager.tokenAddress();
    }
    const managerCounts = {};
    const tokenCounts = {};
    tokenManagers.forEach(function (x) { tokenCounts[x.tokenAddress] = (tokenCounts[x.tokenAddress] || 0) + 1; });
    tokenManagers.forEach(function (x) { managerCounts[x[1]] = (managerCounts[x[1]] || 0) + 1; });
    duplicated = tokenManagers.filter((x) => tokenCounts[x.tokenAddress] > managerCounts[x[1]]);
    console.log(duplicated);

    const mintBurn = tokenManagers.filter((x) => x[2] == 2);
    console.log(`MintBurn: ${mintBurn.length}`);

    const mintBurnFrom = tokenManagers.filter((x) => x[2] == 1);
    console.log(`MintBurnFrom: ${mintBurnFrom.length}`);
}

(async() => {   

    for(const name of Object.keys(info.chains)) {
        console.log(name);
        try {
            await getTokens(name);
        } catch (e) {
            console.log(name);
            console.log(e);
        }
    }
})();