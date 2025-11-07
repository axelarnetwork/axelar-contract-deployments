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

    // This field is used to store the supply tracking status of the token.
    // It is set to true for tokens that are of type `nativeInterchainToken` and
    // their minter address is the zero address.
    trackSupply?: boolean;

    // These fields are used to store the registration status and alignment status of the token.
    registered?: boolean;

    // This field is used to store the alignment status of the token.
    // By default set to true after the token is registered to make sure to run the alignment command
    // after the ITS contracts migration to `v2.2.0`.
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
    const untracked = tokenData.chains.filter((chain) => !chain.trackSupply);
    if (untracked.length === 1) {
        printInfo(`Untracked token ${tokenData.tokenId} on ${untracked[0].axelarChainId}`);
        return untracked[0].axelarChainId;
    }

    // Use ethereum as the origin chain if the token is deployed on any of the Ethereum chains.
    const ethereumChains = ['ethereum', 'core-ethereum', 'ethereum-sepolia', 'core-ethereum-sepolia', 'eth-sepolia'];
    const ethereumChain = tokenData.chains.find((chain) => ethereumChains.includes(chain.axelarChainId.toLowerCase()));
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
    const { env, tokenIds, chains } = options;
    const tokenIdsToProcess = new Set(tokenIds);
    const chainsToProcess = new Set(chains);
    const tokenInfoString = fs.readFileSync(`axelar-chains-config/info/tokens-p2p/tokens-${env}.json`, 'utf8');
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
                            chain.axelarChainId.toLowerCase() !== tokenData.originAxelarChainId?.toLowerCase() &&
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
    if (!options.dryRun) {
        fs.writeFileSync(`axelar-chains-config/info/tokens-p2p/tokens-${env}.json`, JSON.stringify(tokenInfo, null, 2));
    }
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
                supply: tokenOnChain.trackSupply
                    ? await getSupply(tokenOnChain.tokenAddress, config.getChainConfig(tokenOnChain.axelarChainId.toLowerCase()).rpc)
                    : undefined,
                chainName: tokenOnChain.axelarChainId.toLowerCase(),
            } as TokenDataToRegister;
            await registerToken(config, interchainTokenServiceAddress, client, tokenDataToRegister, options.dryRun);
            if (!options.dryRun) {
                tokenOnChain.registered = true;
                tokenOnChain.needsAlignment = true;
            }
            printInfo(`Token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId} is registered`);
        } catch (e) {
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
                config,
                client,
                interchainTokenServiceAddress,
                tokenData.tokenId,
                tokenOnChain.axelarChainId.toLowerCase(),
            );
            if (!options.dryRun) {
                tokenOnChain.registered = registered ? true : false;
            }
            printInfo(`Token ${tokenData.tokenId} on ${tokenOnChain.axelarChainId} is ${registered ? 'registered' : 'not registered'}`);
        } catch (e) {
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
            'The script will register the P2P tokens to the ITS Hub or check if they are already registered.\n' +
                'Requires the token file to be present in the following path:\n' +
                ' * `axelar-chains-config/info/tokens-p2p/tokens-${env}.json`\n' +
                'The tokens file should follow the Squid config format.\n',
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
        .description('Check tokens registration status on the ITS Hub.')
        .addOption(new Option('-chains, --chains <chains...>', 'chains to run the script for. Default: all chains').env('CHAINS'))
        .addOption(
            new Option('-tokenIds, --tokenIds <tokenIds...>', 'tokenIds to run the script for. Default: all tokens').env('TOKEN_IDS'),
        )
        .addOption(new Option('-env, --env <env>', 'environment to run the script for').env('ENV').makeOptionMandatory(true))
        .action((options) => {
            mainQueryProcessor(checkTokensRegistrationInFile, options, []);
        });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
