import 'dotenv/config';
import { Contract, constants, getDefaultProvider, providers } from 'ethers';
import * as fs from 'fs';
import * as path from 'path';

import { tokenManagerTypes } from '../common';
import { printError, printInfo, printWarn } from '../common/utils';
import { SquidTokenData, SquidTokenInfoFile, SquidTokenManagerType } from './register-its-token';

// eslint-disable-next-line @typescript-eslint/no-require-imports
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
// eslint-disable-next-line @typescript-eslint/no-require-imports
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
// eslint-disable-next-line @typescript-eslint/no-require-imports
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json');

const env = process.env.ENV;

// Dynamic import for JSON file with environment variable
const infoPath = path.resolve(__dirname, `../axelar-chains-config/info/${env}.json`);
const info = JSON.parse(fs.readFileSync(infoPath, 'utf-8'));

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

const MAX_RETRIES = 3;
const BATCH_SIZE = 30;

// Simple async mutex per tokenId to prevent race conditions
// Uses a chain of promises to ensure sequential access per key
class AsyncMutex {
    private locks: Map<string, Promise<void>> = new Map();

    async acquire(key: string): Promise<() => void> {
        // Get the current lock (if any)
        const prevLock = this.locks.get(key);

        // Create our lock promise
        let release: () => void;
        const ourLock = new Promise<void>((resolve) => {
            release = resolve;
        });

        // Chain our lock to the previous one (if it exists)
        const lockPromise = prevLock ? prevLock.then(() => ourLock) : ourLock;

        // Set our lock in the map (this is atomic)
        this.locks.set(key, lockPromise);

        // Wait for previous locks to complete
        if (prevLock) {
            await prevLock;
        }

        return () => {
            release();
            // Clean up if no one is waiting (this is the last lock)
            // Check if the current lock is our chained promise or just our lock
            const currentLock = this.locks.get(key);
            if (currentLock === lockPromise || currentLock === ourLock) {
                this.locks.delete(key);
            }
        };
    }
}

const tokenMutex = new AsyncMutex();

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

type TokenDataResult = SquidTokenDataWithTokenId[] | Error;

async function getTokenInfo(tokenManagerAddress, tokenManagerType, provider) {
    const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
    const tokenAddress = await tokenManager.tokenAddress();
    const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
    const decimals = await token.decimals();
    const track = tokenManagerType === tokenManagerTypes.NATIVE_INTERCHAIN_TOKEN && (await token.isMinter(constants.AddressZero));
    return { tokenAddress, decimals, track };
}

async function getTokensFromBlock(
    name: string,
    its: Contract,
    filter: providers.Filter,
    startBlockNumber: number,
    eventsLength: number,
    max: number,
    provider: providers.Provider,
): Promise<TokenDataResult> {
    const end = Math.min(startBlockNumber + eventsLength, max);
    if (startBlockNumber > end) {
        return [];
    }
    let lastError = null;
    let events = null;
    for (let i = 0; i < MAX_RETRIES; i++) {
        try {
            events = await its.queryFilter(filter, startBlockNumber, end);
            break;
        } catch (e) {
            lastError = e;
        }
    }
    if (lastError) {
        return lastError;
    }

    for (let i = 0; i < MAX_RETRIES; i++) {
        try {
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
                                `Token ${tokenId} is conflicting for ${name} with interchain token address ${interchainTokenAddress}`,
                            );
                        }

                        return {
                            axelarChainId: name,
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
        } catch (e) {
            lastError = e;
        }
    }
    return lastError;
}

async function getTokensFromChain(name, chainInfo, tokensInfo: SquidTokenInfoFileWithChains) {
    try {
        if (!tokensInfo.chains[name]) {
            tokensInfo.chains[name] = {
                start: 0,
                end: 0,
                max: 0,
                alreadyProcessedPercentage: '0.00',
                rpcs: [chainInfo.rpc],
            };
        }
        const currentChain = tokensInfo.chains[name];
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

        //if ((await provider.getBlock(currentChain.end)).timestamp >= endTimestamp) return;
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
            const tokensPromises: Promise<TokenDataResult>[] = [];
            for (let i = 0; i < BATCH_SIZE; i++) {
                const newEventsPromise: Promise<TokenDataResult> = getTokensFromBlock(
                    name,
                    its,
                    filter,
                    currentChain.end + 1 + i * eventsLength,
                    eventsLength,
                    currentChain.max,
                    provider,
                );
                tokensPromises.push(newEventsPromise);
            }

            const tokenDataResults: TokenDataResult[] = await Promise.all(tokensPromises);
            const error = tokenDataResults.find((data) => data instanceof Error);
            if (error) {
                printError(`Error getting tokens for ${name}: ${error}`);
                return;
            }

            // There's no errors, so we can flatten the array and get the token data
            const tokensData: SquidTokenDataWithTokenId[] = (tokenDataResults as Array<SquidTokenDataWithTokenId[]>).flat();

            // Process tokens with mutex protection to avoid race conditions
            // Multiple chains can process the same tokenId concurrently
            await Promise.all(
                tokensData.map(async (token) => {
                    const tokenId = token.tokenId;
                    const decimals = token.decimals;
                    const release = await tokenMutex.acquire(tokenId);
                    try {
                        if (!tokensInfo?.tokens?.[tokenId]) {
                            tokensInfo.tokens[tokenId] = {
                                tokenId,
                                decimals,
                                tokenType: 'interchain',
                                chains: [] as SquidTokenData[],
                            };
                        }

                        tokensInfo.tokens[tokenId].chains.push(token);
                    } finally {
                        release();
                    }
                }),
            );

            currentChain.end = Math.min(currentChain.end + BATCH_SIZE * eventsLength, currentChain.max);
            currentChain.alreadyProcessedPercentage = ((currentChain.end / currentChain.max) * 100).toFixed(2);
        }
    } catch (e) {
        printError(`Error getting tokens for ${name}: ${e}`);
    }
}

function writeTokensInfoToFile(tokensInfo, filePath) {
    fs.writeFileSync(filePath, JSON.stringify(tokensInfo, null, 2));
}

(async () => {
    let tokensInfo: SquidTokenInfoFileWithChains = {
        chains: {},
        tokens: {},
    };
    const tokensInfoFilePath = `../axelar-chains-config/info/tokens-p2p/tokens-${env}_2.json`;
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

    const promises = [];
    for (const name of Object.keys(info.chains)) {
        const chainInfo = info.chains[name];
        promises.push(getTokensFromChain(name, chainInfo, tokensInfo));
    }

    // Write to the output file every second
    setInterval(() => {
        writeTokensInfoToFile(tokensInfo, tokensInfoFileAbsolutePath);
    }, 1000);

    await Promise.all(promises).then(() => {
        writeTokensInfoToFile(tokensInfo, tokensInfoFileAbsolutePath);
        process.exit(0);
    });
})();
