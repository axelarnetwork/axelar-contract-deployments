import { Mutex } from 'async-mutex';
import 'dotenv/config';
import { Contract, constants, getDefaultProvider, providers } from 'ethers';
import * as fs from 'fs';
import * as path from 'path';

import { tokenManagerTypes } from '../common';
import { ChainConfig, ConfigManager } from '../common/config';
import { printError, printInfo, printWarn } from '../common/utils';
import { getContractJSON, isConsensusChain } from '../evm/utils';
import { SquidTokenData, SquidTokenInfoFile, SquidTokenManagerType } from './register-p2p-tokens';

const IInterchainTokenService = getContractJSON('IInterchainTokenService');
const ITokenManager = getContractJSON('ITokenManager');
const IInterchainToken = getContractJSON('IInterchainToken');

const queryLimit = {
    ethereum: 500000,
    'eth-sepolia': 1000,
    'ethereum-sepolia': 1000,
    'core-ethereum': 1000,
    avalanche: 2047,
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

const MAX_RETRIES = 3;
const BATCH_SIZE = 30;

// Async mutex per tokenId to prevent race conditions
const tokenWriteMutex = new Mutex();

function getTokenManagerTypeString(numericValue: number): SquidTokenManagerType {
    const mapping: Record<number, SquidTokenManagerType> = {
        0: 'nativeInterchainToken',
        1: 'mintBurnFrom',
        2: 'lockUnlock',
        3: 'lockUnlockFee',
        4: 'mintBurn',
    };
    return mapping[numericValue];
}

type SquidTokenDataWithTokenId = SquidTokenData & {
    tokenId: string;
    decimals: number;
    conflictingInterchainTokenAddress?: string;
};

type SquidTokenInfoFileWithChains = SquidTokenInfoFile & {
    chains: {
        [chainName: string]: {
            start: number;
            end: number;
            max: number;
            alreadyProcessedPercentage: string;
            rpcs: string[];
        };
    };
};

async function getTokenInfo(tokenManagerAddress, tokenManagerType, provider) {
    const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
    const tokenAddress = await tokenManager.tokenAddress();
    const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
    const decimals = await token.decimals();
    const track = tokenManagerType === tokenManagerTypes.NATIVE_INTERCHAIN_TOKEN && (await token.isMinter(constants.AddressZero));
    return { tokenAddress, decimals, track };
}

async function runWithRetries<T>(fn: () => Promise<T>): Promise<T> {
    let lastError: Error | null = null;
    for (let i = 0; i < MAX_RETRIES; i++) {
        try {
            return await fn();
        } catch (e) {
            lastError = e;
            const delayMilliseconds = (i + 1) * 1000;
            await new Promise((resolve) => setTimeout(resolve, delayMilliseconds));
        }
    }
    throw new Error(`Failed to execute function after ${MAX_RETRIES} retries: ${lastError}`);
}

async function getTokensFromBlock(
    axelarChainId: string,
    its: Contract,
    filter: providers.Filter,
    startBlockNumber: number,
    eventsLength: number,
    max: number,
    provider: providers.Provider,
): Promise<SquidTokenDataWithTokenId[]> {
    const end = Math.min(startBlockNumber + eventsLength, max);

    if (startBlockNumber > end) {
        return [];
    }

    const events = await runWithRetries(async () => await its.queryFilter(filter, startBlockNumber, end));
    const tokens = await runWithRetries(async () => {
        const tokenData: SquidTokenDataWithTokenId[] = await Promise.all(
            events
                .map((event) => event.args)
                .map(async (event): Promise<SquidTokenDataWithTokenId> => {
                    const tokenId = event[0];
                    const tokenManagerAddress = event[1];
                    const tokenManagerType = event[2];
                    const tokenInfo = await getTokenInfo(tokenManagerAddress, tokenManagerType, provider);
                    const interchainTokenAddress = await its.interchainTokenAddress(tokenId);

                    if (interchainTokenAddress !== tokenInfo.tokenAddress && tokenManagerType === 0) {
                        printWarn(
                            `Token ${tokenId} is conflicting for ${axelarChainId} with interchain token address ${interchainTokenAddress}`,
                        );
                    }

                    return {
                        axelarChainId,
                        tokenId,
                        tokenManager: tokenManagerAddress,
                        tokenManagerType: getTokenManagerTypeString(tokenManagerType) as SquidTokenManagerType,
                        conflictingInterchainTokenAddress:
                            interchainTokenAddress !== tokenInfo.tokenAddress && tokenManagerType === 0 ? interchainTokenAddress : null,
                        ...tokenInfo,
                    } as SquidTokenDataWithTokenId;
                }),
        );
        return tokenData;
    });
    return tokens;
}

async function getTokensFromChain(chain: ChainConfig, tokensInfo: SquidTokenInfoFileWithChains) {
    try {
        if (!tokensInfo.chains[chain.axelarId]) {
            tokensInfo.chains[chain.axelarId] = {
                start: 0,
                end: 0,
                max: 0,
                alreadyProcessedPercentage: '0.00',
                rpcs: [chain.rpc],
            };
        }
        const currentChain = tokensInfo.chains[chain.axelarId];
        if (!chain.contracts.InterchainTokenService) {
            printWarn(`InterchainTokenService contract not found for ${chain.axelarId}`);
            return;
        }
        printInfo(`ITS at ${chain.axelarId} is at`, chain.contracts.InterchainTokenService.address);

        const eventsLength = queryLimit[chain.axelarId.toLowerCase()] || 2048;
        printInfo('processing... ', chain.axelarId);

        const rpc = currentChain?.rpcs?.[0] || chain.rpc;
        if (!rpc) {
            printError(`No RPC for ${chain.axelarId}`);
            return;
        }
        const provider = getDefaultProvider(rpc);
        const its = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, provider);
        currentChain.max = await provider.getBlockNumber();
        const filter = its.filters.TokenManagerDeployed();
        printInfo(`${chain.axelarId} current block number: ${currentChain.max}`);

        // TODO tkulik: find the first block to start from
        // while (max - min > 1) {
        //     const mid = Math.floor((min + max) / 2);
        //     const timestamp = (await provider.getBlock(mid)).timestamp;
        //     if (timestamp > endTimestamp) {
        //         max = mid;
        //     } else {
        //         min = mid;
        //     }
        // }

        while (currentChain.end < currentChain.max) {
            const tokensPromises: Promise<SquidTokenDataWithTokenId[]>[] = [];
            for (let i = 0; i < BATCH_SIZE; i++) {
                const newEventsPromise: Promise<SquidTokenDataWithTokenId[]> = getTokensFromBlock(
                    chain.axelarId,
                    its,
                    filter,
                    currentChain.end + 1 + i * eventsLength,
                    eventsLength,
                    currentChain.max,
                    provider,
                );
                tokensPromises.push(newEventsPromise);
            }

            const tokensData: SquidTokenDataWithTokenId[] = (await Promise.all(tokensPromises)).flat();

            // Process tokens with mutex protection to avoid race conditions
            // Multiple chains can process the same tokenId concurrently
            await Promise.all(
                tokensData.map(async (token) => {
                    const tokenId = token.tokenId;
                    const decimals = token.decimals;
                    tokenWriteMutex.runExclusive(async () => {
                        if (!tokensInfo?.tokens?.[tokenId]) {
                            tokensInfo.tokens[tokenId] = {
                                tokenId,
                                decimals,
                                tokenType: 'interchain',
                                chains: [] as SquidTokenData[],
                            };
                        }

                        tokensInfo.tokens[tokenId].chains.push(token);
                    });
                }),
            );

            currentChain.end = Math.min(currentChain.end + BATCH_SIZE * eventsLength, currentChain.max);
            currentChain.alreadyProcessedPercentage = ((currentChain.end / currentChain.max) * 100).toFixed(2);
        }
    } catch (e) {
        printError(`Error getting tokens for ${chain.axelarId}: ${e}`);
    }
}

function writeTokensInfoToFile(tokensInfo, filePath) {
    fs.writeFileSync(filePath, JSON.stringify(tokensInfo, null, 2));
}

(async () => {
    const env = process.env.ENV;
    const config = new ConfigManager(env);

    let tokensInfo: SquidTokenInfoFileWithChains = {
        chains: {},
        tokens: {},
    };
    const tokensInfoFilePath = `../axelar-chains-config/info/tokens-p2p/tokens-${env}.json`;
    const tokensInfoFileAbsolutePath = path.resolve(__dirname, tokensInfoFilePath);
    try {
        tokensInfo = JSON.parse(fs.readFileSync(tokensInfoFileAbsolutePath, 'utf-8'));
    } catch (e) {
        const dir = path.dirname(tokensInfoFileAbsolutePath);
        if (!fs.existsSync(dir)) {
            fs.mkdirSync(dir, { recursive: true });
        }
        if (!fs.existsSync(tokensInfoFileAbsolutePath)) {
            writeTokensInfoToFile(tokensInfo, tokensInfoFileAbsolutePath);
        }
    }

    const promises = Object.values(config.chains)
        .filter((chain) => {
            try {
                return isConsensusChain(chain);
            } catch (e) {
                printError(`Error getting chain config for ${chain.axelarId} (skipping chain): ${e.message}`);
                return false;
            }
        })
        .map((chain) => {
            return getTokensFromChain(chain, tokensInfo);
        });

    // Write to the output file every second
    setInterval(() => {
        writeTokensInfoToFile(tokensInfo, tokensInfoFileAbsolutePath);
    }, 1000);

    await Promise.all(promises).then(() => {
        writeTokensInfoToFile(tokensInfo, tokensInfoFileAbsolutePath);
        process.exit(0);
    });
})();
