import IInterchainToken from '@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json';
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Command, Option } from 'commander';
import { Contract, getDefaultProvider } from 'ethers';
import fs from 'fs';

import { printError, printInfo } from '../common';
import { ConfigManager } from '../common/config';
import { isConsensusChain } from '../evm/utils';
import { TokenDataToRegister, checkSingleTokenRegistration, registerToken } from './its';
import { ClientManager, mainProcessor, mainQueryProcessor } from './processor';

export type SquidTokenManagerType = 'nativeInterchainToken' | 'mintBurnFrom' | 'lockUnlock' | 'lockUnlockFee' | 'mintBurn';

export type SquidTokenData = {
    axelarChainId: string;
    tokenManager: string;
    tokenManagerType: SquidTokenManagerType;
    tokenAddress: string;
    track?: boolean;
    registered?: boolean;
    needsAlignment?: boolean;
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

function getOriginChain(tokenData: SquidToken) {
    // TODO tkulik: Why?
    // If only a single chain is untracked, use that chain
    const untracked = tokenData.chains.filter((chain) => !chain.track);
    if (untracked.length === 1) {
        printInfo(`Untracked token ${tokenData.tokenId} on ${untracked[0].axelarChainId}`);
        return untracked[0].axelarChainId;
    }

    // Use ethereum as the origin chain if it exists. Using lowercase to avoid case sensitivity issues (see squid config)
    const ethereumChain = tokenData.chains.find((chain) => chain.axelarChainId.toLowerCase() === 'ethereum');
    if (ethereumChain) {
        return ethereumChain.axelarChainId;
    }

    // Use the first chain that shows up.
    return tokenData.chains[0].axelarChainId;
}

async function getSupply(tokenAddress: string, rpc: string): Promise<string> {
    const provider = getDefaultProvider(rpc);
    const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
    const supply = await token.totalSupply();
    return supply.toString();
}

async function forEachTokenInFile(
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
                            (chains ? chainsToProcess.has(chain.axelarChainId.toLowerCase()) : true) &&
                            (chain.track ?? true) &&
                            chain.axelarChainId !== tokenData.originAxelarChainId &&
                            (chain.registered ? !chain.registered : true) &&
                            isConsensusChain(config.getChainConfig(chain.axelarChainId.toLowerCase()))
                        );
                    } catch (e) {
                        printError(`Error getting chain config for ${chain.axelarChainId} (skipping chain): ${e}`);
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

async function registerTokensInFile(client: ClientManager, config: ConfigManager, options, _args, _fee) {
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;

    if (!interchainTokenServiceAddress) {
        throw new Error('InterchainTokenService contract address not found');
    }

    await forEachTokenInFile(config, options, async (tokenData: SquidToken, tokenOnChain: SquidTokenData) => {
        try {
            const tokenDataToRegister = {
                tokenId: tokenData.tokenId,
                originChain: tokenData.originAxelarChainId || getOriginChain(tokenData),
                decimals: tokenData.decimals,
                supply: await getSupply(tokenOnChain.tokenAddress, config.getChainConfig(tokenOnChain.axelarChainId.toLowerCase()).rpc),
                axelarId: tokenOnChain.axelarChainId,
            } as TokenDataToRegister;
            await registerToken(interchainTokenServiceAddress, client, tokenDataToRegister, options.dryRun);
            tokenOnChain.registered = true;
            tokenOnChain.needsAlignment = true;
            printInfo(`Token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId} is registered`);
        } catch (e) {
            tokenOnChain.registered ??= undefined;
            printError(`Error registering token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId}: ${e}`);
        }
    });
}

async function checkTokensRegistrationInFile(client: CosmWasmClient, config: ConfigManager, options, _args, _fee) {
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;

    if (!interchainTokenServiceAddress) {
        throw new Error('InterchainTokenService contract address not found');
    }

    await forEachTokenInFile(config, options, async (tokenData: SquidToken, tokenOnChain: SquidTokenData) => {
        try {
            const registered = await checkSingleTokenRegistration(
                client,
                interchainTokenServiceAddress,
                tokenData.tokenId,
                tokenOnChain.axelarChainId,
            );
            tokenOnChain.registered = registered ? true : false;
            printInfo(`Token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId} is ${registered ? 'registered' : 'not registered'}`);
        } catch (e) {
            tokenOnChain.registered ??= undefined;
            printError(`Error checking token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId}: ${e}`);
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
                'Requires the following environment variables to be set:, ENV, MNEMONIC.\n' +
                'Requires the token file to be present in the following path:\n' +
                ' * for non-squid tokens: ../axelar-chains-config/info/tokens-p2p/tokens-${env}.json\n' +
                ' * for squid tokens: ../axelar-chains-config/info/tokens-p2p/squid-tokens-${env}.json\n' +
                'The script will register the tokens to the ITS Hub or check if they are registered on the chains.\n',
        );

    program
        .command('register-tokens')
        .description('Register tokens to the ITS Hub.')
        .addOption(new Option('-chains, --chains <chains...>', 'chains to run the script for. Default: all chains').env('CHAINS'))
        .addOption(
            new Option('-tokenIds, --tokenIds <tokenIds...>', 'tokenIds to run the script for. Default: all tokens').env('TOKEN_IDS'),
        )
        .addOption(new Option('-dryRun, --dryRun', 'provide to just print out what will happen when running the command.'))
        .addOption(new Option('-env, --env <env>', 'environment to run the script for').env('ENV').makeOptionMandatory(true))
        .addOption(new Option('-squid, --squid', 'use squid tokens'))
        .addOption(
            new Option('-m, --mnemonic <mnemonic>', 'Mnemonic of the InterchainTokenService operator account')
                .makeOptionMandatory(true)
                .env('MNEMONIC'),
        )
        .action((options) => {
            mainProcessor(registerTokensInFile, options, []);
        });

    program
        .command('check-tokens')
        .addOption(new Option('-chains, --chains <chains...>', 'chains to run the script for. Default: all chains').env('CHAINS'))
        .addOption(
            new Option('-tokenIds, --tokenIds <tokenIds...>', 'tokenIds to run the script for. Default: all tokens').env('TOKEN_IDS'),
        )
        .addOption(new Option('-env, --env <env>', 'environment to run the script for').env('ENV').makeOptionMandatory(true))
        .addOption(new Option('-squid, --squid', 'use squid tokens'))
        .action((options) => {
            mainQueryProcessor(checkTokensRegistrationInFile, options, []);
        });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
