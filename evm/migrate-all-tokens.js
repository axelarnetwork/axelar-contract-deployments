require('dotenv').config();
const env = process.env.ENV;
const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);
const { getContractJSON, printError, printInfo, getGasOptions } = require('./utils');
const fs = require('fs');
const yaml = require('yaml');
const { getWallet } = require('./sign-utils');

const ITokenManager = getContractJSON('ITokenManager');
const IInterchainToken = getContractJSON('IInterchainToken');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');

const NATIVE_INTERCHAIN_TOKEN_MANAGER_TYPE = 0;

const RPCs = yaml.parse(fs.readFileSync(`./axelar-chains-config/rpcs/${env}.yaml`, 'utf-8'));

/*
To use this script first configure your .env file as follows

PRIVATE_KEY=${its owner private key}                                                                                         
ENV=${local if you forked the network first, for testing, or mainnet}
CHAINS=${all to migrate everything, or a comma separated list of chains}

I suggest doing a one chain at a time at first, to make sure it doesn't take too long, and then a few at a time.
./axelar-chains-config/tokenManagers.json has all the chains that are valid (had ITS before the update).
*/

async function migrateTokens(name) {
    const chainName = name.toLowerCase();
    const N = 10;

    if (!tokenManagerInfo[name]) return;
    const tokenManagers = tokenManagerInfo[name].tokenManagers;
    const chain = info.chains[chainName];
    if (chain.contracts.InterchainTokenService.skip) return;
    const rpc = env === 'testnet' ? RPCs.node.EVMBridges.find((chain) => chain.name.toLowerCase() === name.toLowerCase()).rpc_addr : info.chains[name].rpc;
    
    printInfo(`RPC: ${rpc}`);
    const provider = getDefaultProvider(rpc);
    const wallet = await getWallet(process.env.PRIVATE_KEY, provider);
    const service = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, wallet);
    const tokenIds = [];
    const gasOptions = await getGasOptions(chain, {});
    for (const index in tokenManagers) {
        const tokenData = tokenManagers[index];
        if(tokenData.skip) continue;
        // event TokenManagerDeployed(tokenId, tokenManager_, tokenManagerType, params);
        const {tokenId, tokenManagerAddress, tokenManagerType } = tokenData;

        if (tokenManagerType === NATIVE_INTERCHAIN_TOKEN_MANAGER_TYPE) {
            try {
                const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
                const tokenAddress = await tokenManager.tokenAddress();
                const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
                if (await token.isMinter(service.address)) {
                    
                    printInfo(`Migrating token with tokenId: ${tokenId}. | ${Number(index) + 1} out of ${tokenManagers.length}`);

                    //await (await service.migrateInterchainToken(tokenId, gasOptions)).wait();
                    tokenIds.push(tokenId);
                } else {
                    tokenData.skip = true;
                }
            } catch (e) {
                printError(`Error migrating tokens for ${name}: ${e.message}`);
                
                // TODO tkulik: check this case:
                printInfo(`Token with tokenId: ${tokenId} seems to be legacy.. | ${Number(index) + 1} out of ${tokenManagers.length}`);
                //tokenData.skip = true;
            }
        } else {
            tokenData.skip = true;
        }

        fs.writeFileSync(`./axelar-chains-config/info/tokenManagers-${env}.json`, JSON.stringify(tokenManagerInfo, null, 2));
    }
    while(tokenIds.length > 0) {
        const data = [];
        const migrating = tokenIds.splice(0, N);
        printInfo(`Migrating tokens: ${migrating}`);
        for (const tokenId of migrating) {
            const tx = await service.populateTransaction.migrateInterchainToken(tokenId);
            data.push(tx.data);
        }
        try {
            await (await service.multicall(data)).wait();
        } catch (e) {
            printError(`Error migrating tokens for ${name}: ${e.message}`);
        }
    }
}

(async () => {
    let chains;
    if(process.env.CHAINS && process.env.CHAINS != 'all') {
        chains = process.env.CHAINS.split(',');
    }
    for (const name of Object.keys(info.chains)) {
        printInfo(`Migrating tokens for ${name}`);
        if(chains && chains.findIndex((chainName) => chainName == name) == -1) {
            continue;
        }
        try {
            await migrateTokens(name);
        } catch (e) {
            printError(`Error migrating tokens for ${name}: ${e.message}`);
        }
    }
})();
