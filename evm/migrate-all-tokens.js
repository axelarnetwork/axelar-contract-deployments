require('dotenv').config();
const env = process.env.ENV;
const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);
const { getContractJSON, printError, printInfo } = require('./utils');
const fs = require('fs');
const toml = require('toml');
const { getWallet } = require('./sign-utils');

const ITokenManager = getContractJSON('ITokenManager');
const IInterchainToken = getContractJSON('IInterchainToken');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');

const NATIVE_INTERCHAIN_TOKEN_MANAGER_TYPE = 0;

/*
To use this script first configure your .env file as follows

PRIVATE_KEY=${its owner private key}                                                                                         
ENV=${local if you forked the network first, for testing, or mainnet}
CHAINS=${all to migrate everything, or a comma separated list of chains}

I suggest doing a one chain at a time at first, to make sure it doesn't take too long, and then a few at a time.
./axelar-chains-config/tokenManagers.json has all the chains that are valid (had ITS before the update).
*/

const RPCs = toml.parse(fs.readFileSync('./axelar-chains-config/info/rpcs.toml', 'utf-8'));

async function migrateTokens(name) {
    const chainName = name.toLowerCase();

    if (!tokenManagerInfo[name]) return;
    const tokenManagers = tokenManagerInfo[name].tokenManagers;
    const chain = info.chains[chainName];
    if (chain.contracts.InterchainTokenService.skip) return;

    const rpc = env === 'mainnet' ? RPCs.axelar_bridge_evm.find((chain) => chain.name.toLowerCase() === name.toLowerCase()).rpc_addr : info.chains[name].rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = await getWallet(process.env.PRIVATE_KEY, provider);
    
    const service = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, wallet);

    for (const index in tokenManagers) {
        const tokenData = tokenManagers[index];
        // event TokenManagerDeployed(tokenId, tokenManager_, tokenManagerType, params);
        const {tokenId, tokenManagerAddress, tokenManagerType } = tokenData;

        if (tokenManagerType === NATIVE_INTERCHAIN_TOKEN_MANAGER_TYPE) {
            try {
                const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
                const tokenAddress = await tokenManager.tokenAddress();
                const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
                if (await token.isMinter(service.address)) {
                    printInfo(`Migrating token with tokenId: ${tokenId}. | ${Number(index) + 1} out of ${tokenManagers.length}`);
                    await service.migrateInterchainToken(tokenId);
                }
            } catch (e) {
                printError(e);

                printInfo(`Token with tokenId: ${tokenId} seems to be legacy.. | ${Number(index) + 1} out of ${tokenManagers.length}`);
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
            printError(e);
        }
    }
})();
