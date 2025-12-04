import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Argument, Command, Option } from 'commander';
import { Contract, constants, getDefaultProvider } from 'ethers';

import { addEnvOption, tokenManagerTypes, validateParameters } from '../common';
import { printInfo } from '../common';
import { ConfigManager } from '../common/config';
import { getContractJSON } from '../evm/utils';
import { ClientManager, mainProcessor, mainQueryProcessor } from './processor';

const IInterchainToken = getContractJSON('IInterchainToken');

export type TokenData = {
    tokenId: string;
    originChain: string;
    decimals: number;
    chainName: string;
};

export async function registerToken(
    config: ConfigManager,
    interchainTokenServiceAddress: string,
    client: ClientManager,
    tokenData: TokenData,
    dryRun: boolean,
) {
    const alreadyRegistered = await checkTokenRegistrationByChain(
        config,
        client,
        interchainTokenServiceAddress,
        tokenData.tokenId,
        tokenData.chainName,
    );
    if (alreadyRegistered) {
        printInfo(`Token ${tokenData.tokenId} on ${tokenData.chainName} is already registered`);
        return;
    }

    const msg = {
        register_p2p_token_instance: {
            chain: config.getChainConfig(tokenData.chainName).axelarId,
            token_id: formatTokenId(tokenData.tokenId),
            origin_chain: config.getChainConfig(tokenData.originChain).axelarId,
            decimals: tokenData.decimals,
            supply: 'untracked',
        },
    };

    const [account] = client.accounts;
    printInfo('Registering token ', JSON.stringify(msg.register_p2p_token_instance));

    if (!dryRun) {
        await client.execute(account.address, interchainTokenServiceAddress, msg, 'auto');
    }
}

export async function checkTokenRegistrationByChain(
    config: ConfigManager,
    client: CosmWasmClient,
    interchainTokenServiceAddress: string,
    tokenId: string,
    chainName: string,
): Promise<boolean> {
    const registered = await client.queryContractSmart(interchainTokenServiceAddress, {
        token_instance: { chain: config.getChainConfig(chainName).axelarId, token_id: formatTokenId(tokenId) },
    });
    return registered;
}

// TODO tkulik: This command will be used to get the supply of the token on the chain.
export async function tokenSupply(tokenAddress: string, rpc: string): Promise<string> {
    const provider = getDefaultProvider(rpc);
    const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
    const supply = await token.totalSupply();
    return supply.toString();
}

// TODO tkulik: This command will be used in the supply alignment command.
export async function isTokenSupplyTracked(tokenManagerType: number, token: Contract): Promise<boolean> {
    return tokenManagerType === tokenManagerTypes.NATIVE_INTERCHAIN_TOKEN && (await token.isMinter(constants.AddressZero));
}

function formatTokenId(tokenAddress: string): string {
    if (tokenAddress.startsWith('0x')) {
        return tokenAddress.slice(2);
    }
    return tokenAddress;
}

async function registerP2pToken(client: ClientManager, config: ConfigManager, options) {
    const { chain, tokenId, originChain, decimals, dryRun } = options;
    const tokenData = {
        tokenId: tokenId,
        originChain: originChain.toLowerCase(),
        decimals: decimals,
        chainName: chain.toLowerCase(),
    };
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;
    validateParameters({
        isNonEmptyString: { interchainTokenServiceAddress },
    });

    await registerToken(config, interchainTokenServiceAddress, client, tokenData, dryRun);
}

async function checkTokenRegistration(client: ClientManager, config: ConfigManager, options) {
    const { tokenId } = options;

    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;
    validateParameters({
        isNonEmptyString: { interchainTokenServiceAddress },
    });

    const registeredChains = (
        await Promise.all(
            Object.keys(config.chains).map(async (axelarChainId: string) => {
                const registered = await checkTokenRegistrationByChain(
                    config,
                    client,
                    interchainTokenServiceAddress,
                    tokenId,
                    axelarChainId,
                );
                if (registered) {
                    return axelarChainId;
                }
            }),
        )
    ).filter(Boolean);

    if (registeredChains.length === 0) {
        printInfo(`Token ${tokenId} is not registered on any chain`);
        return;
    }

    printInfo(`Token ${tokenId} is registered on: ${registeredChains.join(', ')}`);
}

const programHandler = () => {
    const program = new Command();

    program
        .name('ITS p2p token registration')
        .description('Script to perform ITS p2p token registration and check tokens registration status.');

    const registerP2pTokenCmd = program
        .command('register-p2p-token')
        .description('Register a single P2P consensus token to the ITS Hub.')
        .addOption(new Option('--chain <chain>', 'axelar chain id to run the script for').env('CHAIN').makeOptionMandatory(true))
        .addOption(new Option('--tokenId <tokenId>', 'Token ID to register').env('TOKEN_ID').makeOptionMandatory(true))
        .addOption(new Option('--originChain <originChain>', 'Origin chain of the token').env('ORIGIN_CHAIN').makeOptionMandatory(true))
        .addOption(
            new Option('--decimals <decimals>', 'Decimals of the token').env('DECIMALS').makeOptionMandatory(true).argParser(parseInt),
        )
        .addOption(
            new Option('-m, --mnemonic <mnemonic>', 'Mnemonic of the InterchainTokenService operator account')
                .makeOptionMandatory(true)
                .env('MNEMONIC'),
        )
        .addOption(new Option('--dryRun', 'Provide to just print out what will happen when running the command.'))
        .action((options) => {
            mainProcessor(registerP2pToken, options, []);
        });

    addEnvOption(registerP2pTokenCmd);

    const registeredChainsByTokenCmd = program
        .command('registered-chains-by-token')
        .description('Check if a token is registered on a chain.')
        .addArgument(new Argument('tokenId', 'Token ID to check the registration of'))
        .action((tokenId, options) => {
            options.tokenId = tokenId;
            mainQueryProcessor(checkTokenRegistration, options, []);
        });
    addEnvOption(registeredChainsByTokenCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
