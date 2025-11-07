import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Command, Option } from 'commander';

import { printError, printInfo } from '../common';
import { ConfigManager } from '../common/config';
import { ClientManager, mainProcessor, mainQueryProcessor } from './processor';

export type TokenDataToRegister = {
    tokenId: string;
    originChain: string;
    decimals: number;
    supply?: string;
    axelarId: string;
};

export async function registerToken(
    interchainTokenServiceAddress: string,
    client: ClientManager,
    tokenDataToRegister: TokenDataToRegister,
    dryRun: boolean,
) {
    const supply = tokenDataToRegister.supply;
    const supplyParam = supply ? { tracked: String(supply) } : 'untracked';
    const msg = {
        register_p2p_token_instance: {
            chain: tokenDataToRegister.axelarId,
            token_id: formatTokenAddress(tokenDataToRegister.tokenId),
            origin_chain: tokenDataToRegister.originChain,
            decimals: tokenDataToRegister.decimals,
            supply: supplyParam,
        },
    };

    const [account] = await client.accounts;
    printInfo('Registering token ', JSON.stringify(msg.register_p2p_token_instance));

    if (!dryRun) {
        await client.execute(account.address, interchainTokenServiceAddress, msg, 'auto');
    }
}

export async function checkSingleTokenRegistration(
    client: CosmWasmClient,
    interchainTokenServiceAddress: string,
    tokenId: string,
    axelarChainId: string,
): Promise<boolean> {
    const registered = await client.queryContractSmart(interchainTokenServiceAddress, {
        token_instance: { chain: axelarChainId, token_id: formatTokenAddress(tokenId) },
    });
    return registered;
}

function formatTokenAddress(tokenAddress: string): string {
    if (tokenAddress.startsWith('0x')) {
        return tokenAddress.slice(2);
    }
    return tokenAddress;
}

async function registerSingleToken(client: ClientManager, config: ConfigManager, options) {
    const { chain, tokenId, originChain, decimals, supply, dryRun } = options;
    try {
        const tokenDataToRegister = {
            tokenId: tokenId,
            originChain: originChain,
            decimals: decimals,
            supply: supply,
            axelarId: config.getChainConfig(chain).axelarId,
        };
        const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;

        if (!interchainTokenServiceAddress) {
            throw new Error('InterchainTokenService contract address not found');
        }
        await registerToken(interchainTokenServiceAddress, client, tokenDataToRegister, dryRun);
        printInfo(`Token ${tokenId} on ${chain} is registered successfully`);
    } catch (e) {
        printError(`Error registering token ${tokenId} on ${chain}: ${e}`);
    }
}

async function checkTokensRegistration(client: CosmWasmClient, config: ConfigManager, options) {
    const { chains, tokenIds } = options;

    try {
        const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;

        if (!interchainTokenServiceAddress) {
            throw new Error('InterchainTokenService contract address not found');
        }

        await Promise.all(
            tokenIds.flatMap((tokenId) => {
                return chains.map(async (chainName) => {
                    try {
                        const registered = await checkSingleTokenRegistration(
                            client,
                            interchainTokenServiceAddress,
                            tokenId,
                            config.getChainConfig(chainName).axelarId,
                        );
                        printInfo(`Token ${tokenId} on ${chainName} is ${registered ? 'registered' : 'not registered'}`);
                    } catch (e) {
                        printError(`Error checking token ${tokenId} on ${chainName}: ${e}`);
                    }
                });
            }),
        );
    } catch (e) {
        printError(`Error checking tokens registration: ${e}`);
    }
}

const programHandler = () => {
    const program = new Command();

    program
        .name('ITS p2p token registration')
        .version('1.0.0')
        .description('Script to perform ITS p2p token registration and check tokens registration status.');

    program
        .command('register-p2p-token')
        .description('Register a single P2P consensus token to the ITS Hub.')
        .addOption(new Option('-env, --env <env>', 'environment to run the script for').env('ENV').makeOptionMandatory(true))
        .addOption(new Option('-chain, --chain <chain>', 'axelar chain id to run the script for').env('CHAIN').makeOptionMandatory(true))
        .addOption(new Option('-tokenId, --tokenId <tokenId>', 'Token ID to register').env('TOKEN_ID').makeOptionMandatory(true))
        .addOption(
            new Option('-originChain, --originChain <originChain>', 'Origin chain of the token')
                .env('ORIGIN_CHAIN')
                .makeOptionMandatory(true),
        )
        .addOption(
            new Option('-decimals, --decimals <decimals>', 'Decimals of the token')
                .env('DECIMALS')
                .makeOptionMandatory(true)
                .argParser(parseInt),
        )
        .addOption(new Option('-supply, --supply <supply>', 'Supply of the token').env('SUPPLY'))
        .addOption(
            new Option('-m, --mnemonic <mnemonic>', 'Mnemonic of the InterchainTokenService operator account')
                .makeOptionMandatory(true)
                .env('MNEMONIC'),
        )
        .addOption(new Option('-dryRun, --dryRun', 'provide to just print out what will happen when running the command.'))
        .action((options) => {
            mainProcessor(registerSingleToken, options, []);
        });

    program
        .command('check-tokens-registration')
        .description('Check tokens registration status on the ITS Hub for given chains and tokenIds.')
        .addOption(new Option('-env, --env <env>', 'environment to run the script for').env('ENV').makeOptionMandatory(true))
        .addOption(
            new Option('-chains, --chains <chains...>', 'chains to check the registration for').env('CHAINS').makeOptionMandatory(true),
        )
        .addOption(
            new Option('-tokenIds, --tokenIds <tokenIds...>', 'tokenIds to check the registration for')
                .env('TOKEN_IDS')
                .makeOptionMandatory(true),
        )
        .action((options) => {
            mainQueryProcessor(checkTokensRegistration, options, []);
        });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
