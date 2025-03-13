require('dotenv').config();
const env = process.env.ENV;
const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require('../axelar-chains-config/info/tokenManagers.json');
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json');
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/InterchainTokenService.sol/InterchainTokenService.json');
const fs = require('fs');
const toml = require('toml');
const { getWallet } = require('./sign-utils');

/*
To use this script first configure your .env file as follows

PRIVATE_KEY=${its owner private key}                                                                                         
ENV=${local if you forked the network first, for testing, or mainnet}
CHAINS=${all to migrate everything, or a comma separated list of chains}

I suggest doing a one chain at a time at first, to make sure it doesn't take too long, and then a few at a time.
./axelar-chains-config/tokenManagers.json has all the chains that are valid (had ITS before the update).
*/

const RPCs = toml.parse(fs.readFileSync('./axelar-chains-config/info/rpcs.toml', 'utf-8'));
const itsAddresses = {
    ethereum: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    avalanche: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    fantom: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    polygon: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    moonbeam: '0xb5fb4be02232b1bba4dc8f81dc24c26980de9e3c',
    binance: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    arbitrum: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    celo: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    kava: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    filecoin: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    optimism: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    linea: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    base: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    mantle: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    blast: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    fraxtal: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    scroll: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
};

async function migrateTokens(name) {
    const chainName = name.toLowerCase();

    if (!tokenManagerInfo[name]) return;
    const tokenManagers = tokenManagerInfo[name].tokenManagers;
    const chain = info.chains[name];
    if (chain.contracts.InterchainTokenService.skip) return;

    const rpc = env === 'mainnet' ? RPCs.axelar_bridge_evm.find((chain) => chain.name.toLowerCase() === name.toLowerCase()).rpc_addr : info.chains[name].rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = await getWallet(process.env.PRIVATE_KEY, provider);
    
    const service = new Contract(itsAddresses[chainName], IInterchainTokenService.abi, wallet);

    for (const index in tokenManagers) {
        const tokenData = tokenManagers[index];
        // event TokenManagerDeployed(tokenId, tokenManager_, tokenManagerType, params);
        const tokenId = tokenData[0];
        const tokenManagerAddress = tokenData[1];
        const tokenManagerType = tokenData[2];

        if (tokenManagerType === 0) {
            try {
                const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
                const tokenAddress = await tokenManager.tokenAddress();
                const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
                if (await token.isMinter(service.address)) {
                    console.log(`Migrating token with tokenId: %s. | %s out of %s`, tokenId, Number(index) + 1, tokenManagers.length);
                    await service.migrateInterchainToken(tokenId);
                }
            } catch (e) {
                console.log(e);
                console.log(`Token with tokenId: %s does seems to be legacy. | %s out of %s`, tokenId, Number(index) + 1, tokenManagers.length);
            }
        }
    }
}

(async () => {
    let chains;
    if(process.env.CHAINS && process.env.CHAINS != 'all') {
        chains = process.env.CHAINS.split(',');
    }
    for (const name of Object.keys(info.chains)) {
        if(chains && chains.findIndex((chainName) => chainName == name) == -1) {
            continue;
        }
        try {
            await migrateTokens(name);
        } catch (e) {
            console.log(e);
        }
    }
})();
