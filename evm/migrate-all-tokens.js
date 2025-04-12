require('dotenv').config();
const env = process.env.ENV;
const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);
const { getContractJSON, printError, printInfo, printWalletInfo } = require('./utils');
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

async function migrateTokens(chain) {
    const tokenManagers = tokenManagerInfo[chain.name.toLowerCase()]?.tokenManagers;
    if (!tokenManagers) return;

    if (chain.chainType !== 'evm') return;
    if (!chain.contracts.InterchainTokenService?.address) return;

    printInfo(`Migrating tokens for ${chain.axelarId}`);

    const rpc = (env === 'mainnet' || env === 'testnet') ? RPCs.axelar_bridge_evm.find((chainConfig) => chainConfig.name.toLowerCase() === chain.axelarId.toLowerCase()).rpc_addr : chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = await getWallet(process.env.PRIVATE_KEY, provider);
    await printWalletInfo(wallet);

    const service = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, wallet);
    const migrationTxs = [];

    for (const index in tokenManagers) {
        const tokenData = tokenManagers[index];
        // event TokenManagerDeployed(tokenId, tokenManager_, tokenManagerType, params);
        const tokenId = tokenData[0];
        const tokenManagerAddress = tokenData[1];
        const tokenManagerType = tokenData[2];

        if (tokenManagerType === NATIVE_INTERCHAIN_TOKEN_MANAGER_TYPE) {
            try {
                const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
                const tokenAddress = await tokenManager.tokenAddress();
                const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
                if (await token.isMinter(service.address)) {
                    printInfo(`${chain.axelarId}: Migrating token with tokenId: ${tokenId}. | ${Number(index) + 1} out of ${tokenManagers.length}`);
                    // await service.migrateInterchainToken(tokenId);
                    const tx = (await service.populateTransaction.migrateInterchainToken(tokenId)).data;
                    migrationTxs.push(tx);
                }
            } catch (e) {
                printError(e);

                printInfo(`${chain.axelarId}: Token with tokenId: ${tokenId} seems to be legacy.. | ${Number(index) + 1} out of ${tokenManagers.length}`);
            }
        }
    }

    const batchSize = 100;
    for (let i = 0; i * batchSize < migrationTxs.length; i += 1) {
        const txs = migrationTxs.slice(i * batchSize, (i + 1) * batchSize);
        try {
            await service.multicall(txs);
            printInfo(`${chain.axelarId}: Migrated batch ${i + 1} with size ${batchSize}`);
        } catch (e) {
            printError(e);
            printInfo(`${chain.axelarId}: Failed to migrate batch ${i + 1}`);
        }

        await new Promise((resolve) => setTimeout(resolve, 10000));
    }
}

(async () => {
    let chains;
    if(process.env.CHAINS && process.env.CHAINS != 'all') {
        chains = process.env.CHAINS.split(',');
    }
    for (const chain of Object.values(info.chains)) {
        if(chains && chains.findIndex((chainName) => chainName == chain.axelarId.toLowerCase()) == -1) {
            continue;
        }
        try {
            await migrateTokens(chain);
        } catch (e) {
            printError(e);
        }
    }
})();
