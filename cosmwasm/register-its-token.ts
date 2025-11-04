import IInterchainToken from '@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json';
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Command, Option } from 'commander';
import 'dotenv/config';
import { Contract, getDefaultProvider } from 'ethers';
import fs from 'fs';

import { printError, printInfo } from '../common';
import { ConfigManager } from '../common/config';
import { isConsensusChain } from '../evm/utils';
import { ClientManager, mainProcessor, mainQueryProcessor } from './processor';

export type SquidTokenManagerType = 'nativeInterchainToken' | 'mintBurnFrom' | 'lockUnlock' | 'lockUnlockFee' | 'mintBurn';

export type SquidTokenData = {
    axelarChainId: string;
    tokenManager: string;
    tokenManagerType: SquidTokenManagerType;
    tokenAddress: string;
    track?: boolean;
    registered?: boolean;
};

export type SquidToken = {
    tokenId: string;
    decimals: number;
    tokenType: 'interchain' | 'customInterchain' | 'canonical';
    chains: SquidTokenData[];
    originAxelarChainId?: string;
};

export type SquidTokens = {
    [tokenId: string]: SquidToken;
};

export type SquidTokenInfoFile = {
    tokens: SquidTokens;
};

async function getOriginChain(tokenData: SquidToken, client: CosmWasmClient, itsAddress: string) {
    // TODO tkulik: should we skip this chain in such case?
    // if only a single token exists it has to be the origin token
    if (tokenData.chains.length === 1) {
        return tokenData.chains[0].axelarChainId;
    }

    // TODO tkulik: Why?
    // If only a single chain is untracked, use that chain
    const untracked = tokenData.chains.filter((chain) => !chain.track);
    if (untracked.length === 1) {
        printInfo(`Untracked token ${tokenData.tokenId} on ${untracked[0].axelarChainId}`);
        return untracked[0].axelarChainId;
    }

    // Use ethereum as the origin chain if it exists
    const ethereumChain = tokenData.chains.find((chain) => chain.axelarChainId === 'ethereum');
    if (ethereumChain) {
        return ethereumChain.axelarChainId;
    }

    // Use the first chain that shows up.
    return tokenData.chains[0].axelarChainId;
}

type TokenDataToRegister = {
    tokenId: string;
    originChain: string;
    decimals: number;
    track: boolean;
    supply: string;
    axelarId: string;
};

async function getSupply(tokenAddress: string, rpc: string) {
    const provider = getDefaultProvider(rpc);
    const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
    return await token.totalSupply();
}

function isLegacyP2pConsensusToken(config: ConfigManager, tokenData: SquidToken, chainData: SquidTokenData): boolean {
    return tokenData.tokenType === 'interchain' && isConsensusChain(config.getChainConfig(chainData.axelarChainId));
}

async function registerToken(config: ConfigManager, client: ClientManager, tokenDataToRegister: TokenDataToRegister, dryRun: boolean) {
    const supply = tokenDataToRegister.supply;
    const supplyParam = supply ? { tracked: String(supply) } : 'untracked';
    const msg = {
        register_p2p_token_instance: {
            chain: tokenDataToRegister.axelarId,
            token_id: tokenDataToRegister.tokenId.slice(2),
            origin_chain: config.chains[tokenDataToRegister.originChain].axelarId,
            decimals: tokenDataToRegister.decimals,
            supply: supplyParam,
        },
    };

    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;
    if (!interchainTokenServiceAddress) {
        throw new Error('InterchainTokenService contract address not found');
    }

    const [account] = await client.accounts;
    printInfo('Registering token ', JSON.stringify(msg.register_p2p_token_instance));

    // TODO tkulik: uncomment to implement the registration
    // if (!dryRun) {
    //     await client.execute(account.address, interchainTokenServiceAddress, msg, 'auto');
    // }
}

async function forEachToken(
    config: ConfigManager,
    options,
    processToken: (tokenData: SquidToken, tokenOnChain: SquidTokenData) => Promise<void>,
) {
    const { env, tokenIds, chains, squid } = options;
    const tokenIdsToProcess = new Set(tokenIds);
    const chainsToProcess = new Set(chains);
    const tokenInfoString = fs.readFileSync(
        `axelar-chains-config/info/tokens-p2p/${squid ? 'squid-tokens' : 'tokens'}-${env}.json`,
        'utf8',
    );
    const tokenInfo = JSON.parse(tokenInfoString) as SquidTokenInfoFile;
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;
    if (!interchainTokenServiceAddress) {
        throw new Error('InterchainTokenService contract address not found');
    }
    const promises = Object.values(tokenInfo.tokens)
        .filter((tokenData: SquidToken) => (tokenIds ? tokenIdsToProcess.has(tokenData.tokenId) : true))
        .flatMap((tokenData: SquidToken) => {
            return tokenData.chains
                .filter((chain: SquidTokenData) => {
                    try {
                        return (
                            tokenData.tokenType === 'interchain' &&
                            (chains ? chainsToProcess.has(chain.axelarChainId) : true) &&
                            (chain.track ? chain.track : true) &&
                            chain.axelarChainId !== tokenData.originAxelarChainId &&
                            (chain.registered ? !chain.registered : true) &&
                            isConsensusChain(config.getChainConfig(chain.axelarChainId))
                        );
                    } catch (e) {
                        printError(`Error getting chain config for ${chain.axelarChainId} (skipping chain): ${e.message}`);
                        return false;
                    }
                })
                .map(async (tokenOnChain: SquidTokenData) => {
                    return processToken(tokenData, tokenOnChain);
                });
        });
    await Promise.all(promises);
    fs.writeFileSync(
        `axelar-chains-config/info/tokens-p2p/${squid ? 'squid-tokens' : 'tokens'}-${env}.json`,
        JSON.stringify(tokenInfo, null, 2),
    );
}

async function processTokens(client: ClientManager, config: ConfigManager, options, _args, _fee) {
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;
    if (!interchainTokenServiceAddress) {
        throw new Error('InterchainTokenService contract address not found');
    }
    forEachToken(config, options, async (tokenData: SquidToken, tokenOnChain: SquidTokenData) => {
        try {
            const tokenDataToRegister = {
                tokenId: tokenData.tokenId,
                originChain: tokenData.originAxelarChainId || (await getOriginChain(tokenData, client, interchainTokenServiceAddress)),
                decimals: tokenData.decimals,
                track: tokenOnChain.track,
                supply: await getSupply(tokenOnChain.tokenAddress, config.chains[tokenOnChain.axelarChainId].rpc),
                axelarId: config.chains[tokenOnChain.axelarChainId].axelarId,
            } as TokenDataToRegister;
            await registerToken(config, client, tokenDataToRegister, options.dryRun);
            tokenOnChain.registered = true;
            printInfo(`Token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId} is registered`);
        } catch (e) {
            tokenOnChain.registered ??= undefined;
            printError(`Error registering token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId}: ${e.message}`);
        }
    });
}

async function checkTokensRegistration(client: CosmWasmClient, config: ConfigManager, options, _args, _fee) {
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;
    if (!interchainTokenServiceAddress) {
        throw new Error('InterchainTokenService contract address not found');
    }
    forEachToken(config, options, async (tokenData: SquidToken, tokenOnChain: SquidTokenData) => {
        try {
            const registered = await client.queryContractSmart(interchainTokenServiceAddress, {
                token_instance: { chain: tokenOnChain.axelarChainId, token_id: tokenData.tokenId.slice(2) },
            });
            tokenOnChain.registered = registered ? true : false;
            printInfo(`Token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId} is ${registered ? 'registered' : 'not registered'}`);
        } catch (e) {
            tokenOnChain.registered ??= undefined;
            printError(`Error checking token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId}: ${e.message}`);
        }
    });
}

const programHandler = () => {
    const program = new Command();

    program
        .name('ITS p2p token migration script')
        .version('1.0.0')
        .description(
            'Script to perform ITS p2p token migration.\n' +
                'Requires the following environment variables to be set: CHAINS, TOKEN_IDS, ENV, MNEMONIC.\n' +
                'Requires the token file to be present in the following path:\n' +
                ' * for non-squid tokens: ../axelar-chains-config/info/tokens-p2p/tokens-${env}.json\n' +
                ' * for squid tokens: ../axelar-chains-config/info/tokens-p2p/squid-tokens-${env}.json\n' +
                'The script will register the tokens to the ITS Hub or check if they are registered on the chains.\n',
        );

    program
        .command('register-its-token')
        .description('Register tokens to the ITS Hub.')
        .addOption(new Option('-chains, --chains <chains...>', 'chains to run the script for').env('CHAINS'))
        .addOption(new Option('-tokenIds, --tokenIds <tokenIds...>', 'tokenIds to run the script for').env('TOKEN_IDS'))
        .addOption(new Option('-dryRun, --dryRun', 'provide to just print out what will happen when running the command.'))
        .addOption(new Option('-env, --env <env>', 'environment to run the script for').env('ENV'))
        .addOption(new Option('-squid, --squid', 'use squid tokens'))
        .addOption(
            new Option('-m, --mnemonic <mnemonic>', 'Mnemonic of the InterchainTokenService account')
                .makeOptionMandatory(true)
                .env('MNEMONIC'),
        )
        .action((options) => {
            mainProcessor(processTokens, options, []);
        });

    program
        .command('check-tokens-registration')
        .addOption(new Option('-chains, --chains <chains...>', 'chains to run the script for').env('CHAINS'))
        .addOption(new Option('-tokenIds, --tokenIds <tokenIds...>', 'tokenIds to run the script for').env('TOKEN_IDS'))
        .addOption(new Option('-env, --env <env>', 'environment to run the script for').env('ENV'))
        .addOption(new Option('-squid, --squid', 'use squid tokens'))
        .action((options) => {
            mainQueryProcessor(checkTokensRegistration, options, []);
        });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
