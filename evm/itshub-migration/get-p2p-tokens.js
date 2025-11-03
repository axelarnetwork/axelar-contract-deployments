require('dotenv').config();
const env = process.env.ENV;

const { ethers } = require('hardhat');
const {
    Contract,
    getDefaultProvider,
    constants: { AddressZero },
} = ethers;
const info = require(`../../axelar-chains-config/info/${env}.json`);
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json');
const fs = require('fs');
const path = require('path');
const { printInfo, printError, printWarn } = require('../../common/utils');
const { tokenManagerTypes } = require('../../common');

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

const MAX_RETRIES = 30;
const BATCH_SIZE = 30;

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
        return { success: null, error: null };
    }
    let lastError = null;
    for (let i = 0; i < MAX_RETRIES; i++) {
        try {
            const events = await its.queryFilter(filter, startBlockNumber, end);
            return await Promise.all(
                events
                    .map((event) => event.args)
                    .map(async (event) => {
                        const tokenId = event[0];
                        const tokenManagerAddress = event[1];
                        const tokenManagerType = event[2];
                        const deployParams = event[3];
                        const tokenInfo = await getTokenInfo(tokenManagerAddress, tokenManagerType, provider);

                        if (tokenInfo) {
                            const interchainTokenAddress = await its.interchainTokenAddress(tokenId);
                            if (interchainTokenAddress !== tokenInfo.tokenAddress && tokenManagerType === 0) {
                                printWarn(
                                    `Token ${tokenId} is conflicting for ${name} with interchain token address ${interchainTokenAddress}`,
                                );
                                return {
                                    success: {
                                        tokenId,
                                        tokenManagerAddress,
                                        tokenManagerType,
                                        deployParams,
                                        tokenInfo: {
                                            conflicting: {
                                                interchainTokenAddress,
                                            },
                                            ...tokenInfo,
                                        },
                                    },
                                    error: null,
                                };
                            } else {
                                return {
                                    success: { tokenId, tokenManagerAddress, tokenManagerType, deployParams, tokenInfo: { ...tokenInfo } },
                                    error: null,
                                };
                            }
                        }
                        return { success: { tokenId, tokenManagerAddress, tokenManagerType, deployParams }, error: null };
                    }),
            );
        } catch (e) {
            lastError = e;
        }
    }
    return { success: null, error: lastError };
}

async function getTokensFromChain(name, chainInfo, tokenManagerInfo) {
    try {
        if (!tokenManagerInfo.chains[name]) {
            tokenManagerInfo.chains[name] = {
                start: 0,
                end: 0,
                max: 0,
                alreadyProcessedPercentage: 0,
                rpcs: [chainInfo.rpc],
            };
        }
        let currentChain = tokenManagerInfo.chains[name];
        if (!chainInfo.contracts.InterchainTokenService || chainInfo.contracts.InterchainTokenService.skip) {
            return;
        }
        printInfo(`ITS at ${name} is at`, chainInfo.contracts.InterchainTokenService.address);

        const eventsLength = queryLimit[name.toLowerCase()] || 2048;
        printInfo('processing... ', name);

        const rpc = currentChain?.rpcs?.[0] || chainInfo.rpc;
        if (!rpc) {
            printError(`No RPC for ${name}`);
            return;
        }
        const provider = getDefaultProvider(rpc);
        const its = new Contract(chainInfo.contracts.InterchainTokenService.address, IInterchainTokenService.abi, provider);
        currentChain.max = await provider.getBlockNumber();
        const filter = its.filters.TokenManagerDeployed();
        printInfo(`${name} current block number: ${currentChain.max}`);

        // printInfo(`Trying to request token managers from block ${name}`);
        // try {
        //     its.queryFilter(filter, currentChain.end, currentChain.end + 1);
        // } catch (e) {
        //     printError(`Error requesting token managers from block ${name}: ${e.message}`);
        //     return false;
        // }

        //if ((await provider.getBlock(currentChain.end)).timestamp >= endTimestamp) return;

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

        while (currentChain.end < currentChain.max) {
            let eventsPromises = [];
            for (let i = 0; i < BATCH_SIZE; i++) {
                const newEventsPromise = getTokenManagersFromBlock(
                    name,
                    its,
                    filter,
                    currentChain.end + 1 + i * eventsLength,
                    eventsLength,
                    currentChain.max,
                    provider,
                );
                eventsPromises.push(newEventsPromise);
            }

            const tokenManagerData = await Promise.all(eventsPromises);
            if (tokenManagerData.some((data) => data.error)) {
                printError(`Error getting token managers for ${name}: ${tokenManagerData.find((data) => data.error)?.error?.message}`);
                return;
            }

            for (const data of tokenManagerData.filter((data) => data.success).flat()) {
                printInfo(`Processing token manager ${data.tokenId} for ${name}`);
                const tokenId = data.tokenId;
                if (!tokenManagerInfo?.tokens?.[tokenId]) {
                    tokenManagerInfo.tokens[tokenId] = {};
                    tokenManagerInfo.tokens[tokenId][name] = {
                        conflictingIds: [],
                    };
                }
                if (tokenManagerInfo.tokens[tokenId][name]) {
                    printWarn(`Token ${tokenId} already exists for ${name}`);
                    // TODO tkulik: potentially not thread safe
                    tokenManagerInfo.tokens[tokenId][name].conflictingIds.push({
                        ...data,
                    });
                } else {
                    tokenManagerInfo.tokens[tokenId][name] = {
                        ...data,
                    };
                }
            }

            currentChain.end = Math.min(currentChain.end + BATCH_SIZE * eventsLength, currentChain.max);
            currentChain.alreadyProcessedPercentage = ((currentChain.end / currentChain.max) * 100).toFixed(2);
        }
    } catch (e) {
        printError(`Error getting token managers for ${name}: ${e}`);
    }
}

(async () => {
    let tokenManagerInfo = {
        chains: {},
        tokens: {},
    };
    const tokenManagerInfoFilePath = `../../axelar-chains-config/info/tokens-p2p/tokens-${env}_2.json`;
    const tokenManagerInfoFileAbsolutePath = path.resolve(__dirname, tokenManagerInfoFilePath);
    try {
        tokenManagerInfo = require(tokenManagerInfoFilePath);
    } catch (e) {
        printInfo(`No token manager info file found for ${env}`);
        const dir = path.dirname(tokenManagerInfoFileAbsolutePath);
        if (!fs.existsSync(dir)) {
            fs.mkdirSync(dir, { recursive: true });
        }
        if (!fs.existsSync(tokenManagerInfoFileAbsolutePath)) {
            fs.writeFileSync(tokenManagerInfoFileAbsolutePath, JSON.stringify(tokenManagerInfo, null, 2));
        }
    }

    const promises = [];
    for (const name of Object.keys(info.chains)) {
        const chainInfo = info.chains[name];
        promises.push(getTokensFromChain(name, chainInfo, tokenManagerInfo));
    }

    // write to file every second
    setInterval(() => {
        fs.writeFileSync(tokenManagerInfoFileAbsolutePath, JSON.stringify(tokenManagerInfo, null, 2));
    }, 1000);

    await Promise.all(promises).then(() => {
        fs.writeFileSync(tokenManagerInfoFileAbsolutePath, JSON.stringify(tokenManagerInfo, null, 2));
        process.exit(0);
    });
})();
