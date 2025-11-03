require('dotenv').config();
const env = process.env.ENV;

const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json');
const fs = require('fs');
const { printInfo, printError, printWarn } = require('../../common/utils');
const { tokenManagerTypes } = require('../../common');

// This is before the its was deployed on mainnet.
const startTimestamp = 1702800000;
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

async function getTokenInfo(tokenManagerAddress, tokenManagerType, provider) {
    const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
    const tokenAddress = await tokenManager.tokenAddress();
    const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
    const decimals = await token.decimals();
    const track = tokenManagerType === tokenManagerTypes.NATIVE_INTERCHAIN_TOKEN && (await token.isMinter(AddressZero));
    return { tokenAddress, decimals, track };
}

async function getTokenManagersFromBlock(name, its, filter, startBlockNumber, eventsLength, max, provider) {
    const end = Math.min(startBlockNumber + eventsLength, max);
    if (startBlockNumber > end) {
        return [];
    }
    for (let i = 0; i < 30; i++) {
        try {
            const events = await its.queryFilter(filter, startBlockNumber, end);
            return await Promise.all(
                events.map(async (event) => {
                    const tokenInfo = await getTokenInfo(event.args[1], event.args[2], provider);
                    const tokenId = event.args[0];
                    const tokenManagerAddress = event.args[1];
                    const tokenManagerType = event.args[2];
                    const deployParams = event.args[3];
                    const interchainTokenAddress = await its.interchainTokenAddress(tokenId);

                    if (interchainTokenAddress !== tokenInfo.tokenAddress && tokenManagerType === 0) {
                        printWarn(`Token ${tokenId} is conflicting for ${name} with interchain token address ${interchainTokenAddress}`);
                        return {
                            tokenId,
                            tokenManagerAddress,
                            tokenManagerType,
                            deployParams,
                            ...tokenInfo,
                            conflicting: {
                                interchainTokenAddress,
                            },
                        };
                    }
                    return { tokenId, tokenManagerAddress, tokenManagerType, deployParams, ...tokenInfo };
                }),
            );
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

        const rpc = tokenManagerInfo[name].rpcs[0] || chain.rpc;
        printInfo(name, rpc);
        if (!rpc) {
            printError(`No RPC for ${name}`);
            return false;
        }
        const provider = getDefaultProvider(rpc);
        const its = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, provider);
        const max = await provider.getBlockNumber();

        if (!tokenManagerInfo[name]) {
            tokenManagerInfo[name] = {
                start: 0,
                end: 0,
                max: max,
                alreadyProcessedPercentage: 0,
                rpcs: [rpc],
            };
        }

        const filter = its.filters.TokenManagerDeployed();
        printInfo(`${name} current block number: ${max}`);

        printInfo(`Trying to request token managers from block ${name}`);
        try {
            its.queryFilter(filter, tokenManagerInfo[name].end, tokenManagerInfo[name].end + 1);
        } catch (e) {
            printError(`Error requesting token managers from block ${name}: ${e.message}`);
            return false;
        }

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
                    provider,
                );
                eventsPromises = eventsPromises.concat(newEventsPromises);
            }

            const tokenManagerData = await Promise.all(eventsPromises);
            if (tokenManagerData.includes(false)) {
                printError(`Failed to get token managers for ${name} after 30 tries`);
                return false;
            }

            for (const tokenManagerData of tokenManagerData.flat()) {
                const tokenId = tokenManagerData.tokenId;
                if (!tokenManagerInfo.tokens[tokenId]) {
                    tokenManagerInfo.tokens[tokenId] = {};
                    tokenManagerInfo.tokens[tokenId][name] = {
                        conflicting: [],
                    };
                }
                if (tokenManagerInfo.tokens[tokenId][name]) {
                    printWarn(`Token ${tokenId} already exists for ${name}`);
                    tokenManagerInfo.tokens[tokenId][name].conflicting.push({
                        ...tokenManagerData,
                    });
                }
                tokenManagerInfo.tokens[tokenId][name] = {
                    ...tokenManagerData,
                };
            }

            tokenManagerInfo[name].end = Math.min(tokenManagerInfo[name].end + batchSize * eventsLength, tokenManagerInfo[name].max);
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
    let tokenManagerInfo = {};
    const tokenManagerInfoFilePath = `../axelar-chains-config/info/tokens-p2p/tokens-${env}_2.json`;
    try {
        tokenManagerInfo = require(tokenManagerInfoFilePath);
    } catch (e) {
        printInfo(`No token manager info file found for ${env}`);
    }

    const promises = [];
    for (const name of Object.keys(info.chains)) {
        promises.push(getTokenManagers(name, tokenManagerInfo));
    }
    // write to file every second
    setInterval(() => {
        fs.writeFileSync(tokenManagerInfoFilePath, JSON.stringify(tokenManagerInfo, null, 2));
    }, 1000);

    await Promise.all(promises).then(() => {
        fs.writeFileSync(tokenManagerInfoFilePath, JSON.stringify(tokenManagerInfo, null, 2));
        process.exit(0);
    });
})();
