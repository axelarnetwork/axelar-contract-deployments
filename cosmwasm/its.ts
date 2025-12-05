import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Argument, Command, Option } from 'commander';
import { Contract, constants, getDefaultProvider } from 'ethers';

import { addEnvOption, validateParameters } from '../common';
import { printInfo } from '../common';
import { ConfigManager } from '../common/config';
import { getContractJSON } from '../evm/utils';
import { ClientManager, mainProcessor, mainQueryProcessor } from './processor';

const IInterchainToken = getContractJSON('IInterchainToken');

export type TokenInstance = {
    supply: 'untracked' | { tracked: string };
    decimals: number;
};

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
    const alreadyRegistered = await tokenInstanceByChain(
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

export async function tokenInstanceByChain(
    config: ConfigManager,
    client: CosmWasmClient,
    interchainTokenServiceAddress: string,
    tokenId: string,
    chainName: string,
): Promise<TokenInstance> {
    const registered = await client.queryContractSmart(interchainTokenServiceAddress, {
        token_instance: { chain: config.getChainConfig(chainName).axelarId, token_id: formatTokenId(tokenId) },
    });
    return registered;
}

export async function alignTokenSupplyOnHub(
    client: ClientManager,
    config: ConfigManager,
    interchainTokenServiceAddress: string,
    tokenId: string,
    tokenAddress: string,
    chain: string,
    dryRun: boolean,
) {
    const tokenInstance = await tokenInstanceByChain(config, client, interchainTokenServiceAddress, tokenId, chain);
    if (!tokenInstance) {
        printInfo(`Token ${tokenId} on ${chain} is not registered`);
        return;
    }

    const { origin_chain } = await client.queryContractSmart(interchainTokenServiceAddress, {
        token_config: { token_id: formatTokenId(tokenId) },
    });

    if (origin_chain === config.getChainConfig(chain).axelarId) {
        printInfo(`Token ${tokenId} origin chain is ${chain}, it should be set to untracked.`);
        return;
    }

    const { supply, isTokenSupplyTracked } = await tokenSupplyByChain(tokenAddress, config.getChainConfig(chain).rpc);

    if (!isTokenSupplyTracked) {
        printInfo(`Token ${tokenId} on ${chain} supply should not be tracked`);
        return;
    }

    let supplyOnHub: bigint;
    if (tokenInstance.supply === 'untracked') {
        supplyOnHub = BigInt(0);
    } else {
        supplyOnHub = BigInt(tokenInstance.supply.tracked);
    }

    if (supply === supplyOnHub) {
        printInfo(`Token ${tokenId} on ${chain} supply is up-to-date`);
        return;
    }

    const supplyModifier = supply > supplyOnHub ? 'increase_supply' : 'decrease_supply';
    const supplyDifference = supply > supplyOnHub ? supply - supplyOnHub : supplyOnHub - supply;

    const msg = {
        modify_supply: {
            chain: config.getChainConfig(chain).axelarId,
            token_id: formatTokenId(tokenId),
            supply_modifier: {
                [supplyModifier]: supplyDifference.toString(),
            },
        },
    };

    const [account] = client.accounts;
    printInfo('Aligning token supply ', JSON.stringify(msg.modify_supply));

    if (!dryRun) {
        await client.execute(account.address, interchainTokenServiceAddress, msg, 'auto');
    }
}

export async function isTokenSupplyTracked(token: Contract): Promise<boolean> {
    return await token.isMinter(constants.AddressZero);
}

export async function tokenSupplyByChain(tokenAddress: string, rpc: string): Promise<{ supply: bigint; isTokenSupplyTracked: boolean }> {
    const provider = getDefaultProvider(rpc);
    const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
    const supply = await token.totalSupply();
    return {
        supply: BigInt(supply.toString()),
        isTokenSupplyTracked: await isTokenSupplyTracked(token),
    };
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
                const registered = await tokenInstanceByChain(config, client, interchainTokenServiceAddress, tokenId, axelarChainId);
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

async function alignTokenSupply(client: ClientManager, config: ConfigManager, options) {
    const { tokenId, tokenAddress, chain, dryRun } = options;
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;
    validateParameters({
        isNonEmptyString: { interchainTokenServiceAddress },
    });

    await alignTokenSupplyOnHub(client, config, interchainTokenServiceAddress, tokenId, tokenAddress, chain, dryRun);
}

const programHandler = () => {
    const program = new Command();

    program
        .name('ITS p2p token registration')
        .description('Script to perform ITS p2p token registration and check tokens registration status.');

    const registerP2pTokenCmd = program
        .command('register-p2p-token')
        .description('Register a single P2P consensus token to the ITS Hub.')
        .addOption(new Option('--chain <chain>', 'axelar chain id to run the script for').makeOptionMandatory(true))
        .addOption(new Option('--tokenId <tokenId>', 'Token ID to register').makeOptionMandatory(true))
        .addOption(new Option('--originChain <originChain>', 'Origin chain of the token').makeOptionMandatory(true))
        .addOption(new Option('--decimals <decimals>', 'Decimals of the token').makeOptionMandatory(true).argParser(parseInt))
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

    const alignTokenSupplyCmd = program
        .command('align-token-supply')
        .description('Align the supply of a token on a chain with the supply on the chain.')
        .addOption(new Option('--tokenId <tokenId>', 'Token ID to modify the supply of').makeOptionMandatory(true))
        .addOption(new Option('--chain <chain>', 'Chain to modify the supply of').makeOptionMandatory(true))
        .addOption(new Option('--tokenAddress <tokenAddress>', 'Token address to modify the supply of').makeOptionMandatory(true))
        .addOption(new Option('--dryRun', 'Provide to just print out what will happen when running the command.'))
        .addOption(
            new Option('-m, --mnemonic <mnemonic>', 'Mnemonic of the InterchainTokenService operator account')
                .makeOptionMandatory(true)
                .env('MNEMONIC'),
        )
        .action((options) => {
            mainProcessor(alignTokenSupply, options, []);
        });

    addEnvOption(alignTokenSupplyCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
