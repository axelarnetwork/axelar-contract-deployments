import { Mutex } from 'async-mutex';
import { Command } from 'commander';
import 'dotenv/config';
import { Contract, getDefaultProvider, providers } from 'ethers';
import * as fs from 'fs';
import * as path from 'path';

import { addEnvOption } from '../common/cli-utils';
import { ChainConfig, ConfigManager } from '../common/config';
import { printError, printInfo, printWarn } from '../common/utils';
import { getContractJSON, isConsensusChain } from '../evm/utils';
import { isTokenSupplyTracked } from './its';
import { ClientManager, mainQueryProcessor } from './processor';
import { SquidToken, SquidTokenData, SquidTokenInfoFile } from './register-p2p-tokens';

const IInterchainTokenService = getContractJSON('IInterchainTokenService');
const ITokenManager = getContractJSON('ITokenManager');
const IInterchainToken = getContractJSON('IInterchainToken');

const DEFAULT_QUERY_LIMIT = 2048;

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

function getOriginChain(tokenData: SquidTokenDataWithTokenId[]): string {
    // If only a single chain is untracked, use that chain
    const untracked = tokenData.filter((chain) => !chain.trackSupply);
    if (untracked.length === 1) {
        return untracked[0].axelarChainId;
    }

    // Use ethereum as the origin chain if the token is deployed on any of the Ethereum chains.
    const ethereumChains = ['ethereum', 'core-ethereum', 'ethereum-sepolia', 'core-ethereum-sepolia', 'eth-sepolia'];
    const ethereumChain = tokenData.find((chain) => ethereumChains.includes(chain.axelarChainId.toLowerCase()));
    if (ethereumChain) {
        return ethereumChain.axelarChainId;
    }

    // Use the first chain that shows up.
    return tokenData[0].axelarChainId;
}

type SquidTokenDataWithTokenId = SquidTokenData & {
    tokenId: string;
    decimals: number;
    trackSupply: boolean;
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
    tokens: {
        [tokenId: string]: SquidToken & {
            chains: SquidTokenDataWithTokenId[];
        };
    };
};

async function getTokenInfo(tokenManagerAddress, tokenManagerType, provider) {
    const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
    const tokenAddress = await tokenManager.tokenAddress();
    const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
    let decimals: number | undefined = undefined;
    try {
        decimals = await token.decimals();
    } catch (e) {
        printWarn(`Could not get decimals for ${tokenAddress}: ${e}`);
    }

    const trackSupply = await isTokenSupplyTracked(tokenManagerType, token);
    return { tokenAddress, decimals, trackSupply };
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

        const eventsLength = queryLimit[chain.axelarId.toLowerCase()] || DEFAULT_QUERY_LIMIT;
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

        while (currentChain.end < currentChain.max) {
            const tokensPromises: Promise<SquidTokenDataWithTokenId[]>[] = [];
            for (let i = 0; i < BATCH_SIZE; i++) {
                const startBlockNumber = currentChain.end + 1 + i * (eventsLength + 1);
                const newEventsPromise: Promise<SquidTokenDataWithTokenId[]> = getTokensFromBlock(
                    chain.axelarId,
                    its,
                    filter,
                    startBlockNumber,
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
                    await tokenWriteMutex.runExclusive(async () => {
                        if (!tokensInfo?.tokens?.[tokenId]) {
                            tokensInfo.tokens[tokenId] = {
                                tokenId,
                                originAxelarChainId: token.axelarChainId,
                                decimals,
                                tokenType: 'interchain',
                                chains: [] as SquidTokenDataWithTokenId[],
                            };
                        }

                        tokensInfo.tokens[tokenId].chains.push(token);
                        tokensInfo.tokens[tokenId].originAxelarChainId = getOriginChain(tokensInfo.tokens[tokenId].chains);
                    });
                }),
            );

            currentChain.end = Math.min(currentChain.end + BATCH_SIZE * (eventsLength + 1), currentChain.max);
            currentChain.alreadyProcessedPercentage = ((currentChain.end / currentChain.max) * 100).toFixed(2);
        }
    } catch (e) {
        printError(`Error getting tokens for ${chain.axelarId}: ${e}`);
    }
}

function writeTokensInfoToFile(tokensInfo, filePath) {
    fs.writeFileSync(filePath, JSON.stringify(tokensInfo, null, 2));
}

async function tokenIndexer(_client: ClientManager, config: ConfigManager, options) {
    const { env } = options;
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
    }

    const promises = Object.values(config.chains)
        .filter((chain) => isConsensusChain(chain))
        .map((chain) => getTokensFromChain(chain, tokensInfo));

    // Write to the output file every second
    setInterval(() => {
        writeTokensInfoToFile(tokensInfo, tokensInfoFileAbsolutePath);
    }, 1000);

    await Promise.all(promises).then(() => {
        writeTokensInfoToFile(tokensInfo, tokensInfoFileAbsolutePath);
        process.exit(0);
    });
}

async function programHandler() {
    const program = new Command();
    const tokenIndexerCmd = program
        .name('Get P2P tokens')
        .description('Get P2P tokens from consensus chains.')
        .action((options) => {
            mainQueryProcessor(tokenIndexer, options, []);
        });
    addEnvOption(tokenIndexerCmd);

    program.parse();
}

if (require.main === module) {
    programHandler();
}
