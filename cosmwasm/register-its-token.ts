import IInterchainToken from '@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json';
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Command, Option } from 'commander';
import 'dotenv/config';
import { Contract, getDefaultProvider } from 'ethers';
import fs from 'fs';

import { printError, printInfo } from '../common';
import { ConfigManager } from '../common/config';
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
    // if only a single token exists it has to be the origin token (those will be skipped later).
    if (tokenData.chains.length === 1) {
        return tokenData.chains[0].axelarChainId;
    }

    // TODO tkulik: if the token is already registered, do we need to process it again?
    // if a token is already registered on axelar, use the same origin chain.
    try {
        const originChain = await client.queryContractSmart(itsAddress, {
            token_config: { token_id: tokenData.tokenId.slice(2) },
        });
        if (originChain) {
            return originChain.origin_chain;
        }
    } catch (e) {
        printError(`Error getting origin chain for ${tokenData.tokenId}: ${e.message}`);
    }

    // if only a single chain is untacked, use that chain
    const untracked = tokenData.chains.filter((chain) => !chain.track);
    if (untracked.length === 1) {
        printInfo(`Untracked token ${tokenData.tokenId} on ${untracked[0].axelarChainId}`);
        return untracked[0].axelarChainId;
    }

    // just use the first chain that shows up.
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
    const registered = await client.queryContractSmart(interchainTokenServiceAddress, {
        token_instance: { chain: tokenDataToRegister.axelarId, token_id: tokenDataToRegister.tokenId.slice(2) },
    });
    if (registered) return;

    const [account] = await client.accounts;
    printInfo('Registering token ', JSON.stringify(msg.register_p2p_token_instance));

    // TODO tkulik: implement the registration
    // if (!dryRun) {
    //     await client.execute(account.address, interchainTokenServiceAddress, msg, 'auto');

    //     // TODO tkulik: better to query the chain for the token registration?
    //     // TODO tkulik: create a new command to query the chain for the token registration?
    //     // If registration is successfull skip this token in the future without needing to query.
    //     // tokenIterator.get().registered = true;
    // }
}

async function processTokens(client: ClientManager, config: ConfigManager, options, _args, _fee) {
    const { env, tokenIds, chains, squid } = options;
    const tokenIdsToProcess = new Set(tokenIds);
    const chainsToProcess = new Set(chains);
    const tokenInfoString = fs.readFileSync(
        `../axelar-chains-config/info/tokens-p2p/${squid ? 'squid-tokens' : 'tokens'}-${env}.json`,
        'utf8',
    );
    const tokenInfo = JSON.parse(tokenInfoString) as SquidTokenInfoFile;
    Object.values(tokenInfo.tokens)
        .filter((tokenData: SquidToken) => tokenIdsToProcess.has(tokenData.tokenId))
        .map((tokenData: SquidToken) => {
            return (
                tokenData.chains
                    // TODO tkulik: chain.axelarChainId !== tokenData.originAxelarChainId was taken from the original PR.
                    .filter(
                        (chain) =>
                            chainsToProcess.has(chain.axelarChainId) &&
                            (chain.track ? chain.track : true) &&
                            chain.axelarChainId !== tokenData.originAxelarChainId,
                    )
                    .map(async (tokenOnChain) => {
                        const tokenDataToRegister = {
                            tokenId: tokenData.tokenId,
                            originChain: tokenData.originAxelarChainId,
                            decimals: tokenData.decimals,
                            track: tokenOnChain.track,
                            supply: await getSupply(tokenOnChain.tokenAddress, config.chains[tokenOnChain.axelarChainId].rpc),
                            axelarId: config.chains[tokenOnChain.axelarChainId].axelarId,
                        } as TokenDataToRegister;
                        await registerToken(config, client, tokenDataToRegister, options.dryRun);
                        tokenOnChain.registered = true;
                        printInfo(`Token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId} is registered`);
                    })
            );
        });
    fs.writeFileSync(
        `../axelar-chains-config/info/tokens-p2p/${squid ? 'squid-tokens' : 'tokens'}-${env}.json`,
        JSON.stringify(tokenInfo, null, 2),
    );
}

async function checkTokensRegistration(client: CosmWasmClient, config: ConfigManager, options, _args, _fee) {
    const { env, tokenIds, chains, squid } = options;
    const tokenIdsToProcess = new Set(tokenIds);
    const chainsToProcess = new Set(chains);
    const tokenInfoString = fs.readFileSync(
        `../axelar-chains-config/info/tokens-p2p/${squid ? 'squid-tokens' : 'tokens'}-${env}.json`,
        'utf8',
    );
    const tokenInfo = JSON.parse(tokenInfoString) as SquidTokenInfoFile;
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;
    if (!interchainTokenServiceAddress) {
        throw new Error('InterchainTokenService contract address not found');
    }
    Object.values(tokenInfo.tokens)
        .filter((tokenData: SquidToken) => tokenIdsToProcess.has(tokenData.tokenId))
        .map((tokenData: SquidToken) => {
            return (
                tokenData.chains
                    // TODO tkulik: chain.axelarChainId !== tokenData.originAxelarChainId was taken from the original PR.
                    .filter(
                        (chain) =>
                            chainsToProcess.has(chain.axelarChainId) &&
                            (chain.track ? chain.track : true) &&
                            chain.axelarChainId !== tokenData.originAxelarChainId,
                    )
                    .forEach(async (tokenOnChain) => {
                        const registered = await client.queryContractSmart(interchainTokenServiceAddress, {
                            token_instance: { chain: tokenOnChain.axelarChainId, token_id: tokenData.tokenId.slice(2) },
                        });
                        tokenOnChain.registered = registered ? true : false;
                        printInfo(
                            `Token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId} is ${registered ? 'registered' : 'not registered'}`,
                        );
                    })
            );
        });
    fs.writeFileSync(
        `../axelar-chains-config/info/tokens-p2p/${squid ? 'squid-tokens' : 'tokens'}-${env}.json`,
        JSON.stringify(tokenInfo, null, 2),
    );
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
