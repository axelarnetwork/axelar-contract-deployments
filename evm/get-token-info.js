require('dotenv').config();
const env = process.env.ENV;

const { ethers } = require('hardhat');
const { Contract, getDefaultProvider, constants: { AddressZero } } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json')
const fs = require('fs');
const toml = require('toml');
const { printInfo } = require('../common');

const RPCs = toml.parse(fs.readFileSync(`./axelar-chains-config/rpcs/info/${env}.toml`, 'utf-8'));

async function getTokens(name) {
    try {
        const chain = info.chains[name];
        if (
            !chain.contracts.InterchainTokenService || 
            chain.contracts.InterchainTokenService.skip || 
            !tokenManagerInfo[name] || 
            !tokenManagerInfo[name].tokenManagers
        ) return false;
        printInfo(`ITS at ${name} is at`, chain.contracts.InterchainTokenService.address );

        // if (name != 'mantle') { return; }

        console.log('processing... ', name);

        const rpc = env === 'mainnet' ? RPCs.axelar_bridge_evm.find((chain) => chain.name.toLowerCase() === name).rpc_addr : chain.rpc;
        const provider = getDefaultProvider(rpc);

        const its = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, provider);
        const tokenManagers = tokenManagerInfo[name].tokenManagers;

        let i = 0;
        let tries = 0;
        while (i < tokenManagers.length) {
            try {
                const tokenData = tokenManagers[i];
                if (tokenData.decimals != null) {
                    i++;
                    continue;
                }
                const tokenId = tokenData[0];
                const tokenManagerAddress = tokenData[1];
                const tokenManagerType = tokenData[2];
                
                printInfo(`${name}: Processing (${i}/${tokenManagers.length}), tokenId: ${tokenId}`);
                const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
                const tokenAddress = await tokenManager.tokenAddress();

                const token = new Contract(tokenAddress, IInterchainToken.abi, provider);

                const tokenDecimals = await token.decimals();

                const decimals = tokenDecimals;
                const track = tokenManagerType === 0 && await token.isMinter(AddressZero);
                tokenManagerInfo[name].tokenManagers[i] = {
                    tokenId,
                    tokenManagerAddress,
                    tokenManagerType,
                    deployParams: tokenData[3],
                    tokenAddress,
                    decimals,
                    track,
                }
                fs.writeFileSync(`./axelar-chains-config/info/tokenManagers-${env}.json`, JSON.stringify(tokenManagerInfo, null, 2));
                tries = 0;
                i++;
            } catch (e) {
                tries++;
                if (tries >= 10) {
                    console.log(e);
                    i++;
                }
            }
        }
        return true;
    } catch (e) {
        console.log(name);
        console.log(e);
    }
}

(async () => {
    for (const name of Object.keys(info.chains)) {
        // add an await to run in sequence, which is slower.
        getTokens(name).then((success) => console.log(name, 'returned', success));
        
    }
})();
