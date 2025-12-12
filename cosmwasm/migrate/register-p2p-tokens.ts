import { Command, Option } from 'commander';
import fs from 'fs';

import { addEnvOption, printError } from '../../common';
import { ConfigManager } from '../../common/config';
import { validateParameters } from '../../common/utils';
import { isConsensusChain } from '../../evm/utils';
import { TokenData, alignTokenSupplyOnHub, formatTokenId, registerToken } from '../its';
import { ClientManager, mainProcessor } from '../processor';

export type SquidTokenData = {
    axelarChainId: string;
    tokenAddress: string;
    decimals?: number;
};

export type SquidToken = {
    tokenId: string;
    decimals: number;
    tokenType: 'interchain' | 'customInterchain' | 'canonical';
    chains: SquidTokenData[];
    originAxelarChainId: string;
};

export type SquidTokens = {
    [tokenId: string]: SquidToken;
};

export type SquidTokenInfoFile = {
    tokens: SquidTokens;
};

async function filteredTokens(env: string, tokenIds: string[]): Promise<SquidToken[]> {
    const tokenIdsToProcess = new Set(tokenIds);
    const tokenInfoString = fs.readFileSync(`axelar-chains-config/info/tokens-p2p/tokens-${env}.json`, 'utf8');
    const tokenInfo = JSON.parse(tokenInfoString) as SquidTokenInfoFile;
    return Object.values(tokenInfo.tokens).filter(
        (token: SquidToken) => (tokenIds ? tokenIdsToProcess.has(token.tokenId) : true) && token.tokenType === 'interchain',
    );
}

async function forEachTokenAndChain(
    config: ConfigManager,
    tokens: SquidToken[],
    chains: string[],
    processToken: (token: SquidToken, chain: SquidTokenData) => Promise<void>,
): Promise<boolean> {
    let error = false;
    const chainsToProcess = new Set(chains?.map((chain: string) => chain.toLowerCase()) || []);

    for (const token of tokens) {
        const filteredChains: SquidTokenData[] = token.chains.filter((chain: SquidTokenData) => {
            try {
                return (
                    (chains ? chainsToProcess.has(chain.axelarChainId.toLowerCase()) : true) &&
                    isConsensusChain(config.getChainConfig(chain.axelarChainId.toLowerCase()))
                );
            } catch (e) {
                printError(`Error getting chain config for ${chain.axelarChainId} (skipping chain): ${e}`);
                return false;
            }
        });

        for (const chain of filteredChains) {
            try {
                await processToken(token, chain);
            } catch (e) {
                printError(`Error processing token ${token.tokenId} on ${chain.axelarChainId}: ${e}`);
                error = true;
            }
        }
    }
    return !error;
}

async function registerTokensInFile(client: ClientManager, config: ConfigManager, options, _args, _fee) {
    const { env, tokenIds, chains } = options;
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;
    validateParameters({
        isNonEmptyString: { interchainTokenServiceAddress },
    });

    const tokens: SquidToken[] = await filteredTokens(env, tokenIds);

    const validateTokens = await forEachTokenAndChain(config, tokens, chains, async (token: SquidToken, chain: SquidTokenData) => {
        validateParameters({
            isNonEmptyString: {
                tokenId: token.tokenId,
                originAxelarChainId: token.originAxelarChainId,
                axelarChainId: chain.axelarChainId,
            },
            isNumber: { decimals: chain.decimals ?? token.decimals },
        });
    });

    if (!validateTokens) {
        throw new Error('Error validating tokens');
    }

    const registerTokens = await forEachTokenAndChain(config, tokens, chains, async (token: SquidToken, chain: SquidTokenData) => {
        let originChainFromAxelar = undefined;
        try {
            const { origin_chain } = await client.queryContractSmart(interchainTokenServiceAddress, {
                token_config: { token_id: formatTokenId(token.tokenId) },
            });
            originChainFromAxelar = origin_chain;
        } catch (e) {}
        const tokenData: TokenData = {
            tokenId: token.tokenId,
            originChain: originChainFromAxelar ?? token.originAxelarChainId.toLowerCase(),
            decimals: chain.decimals ?? token.decimals,
            chainName: chain.axelarChainId.toLowerCase(),
        } as TokenData;
        await registerToken(config, interchainTokenServiceAddress, client, tokenData, options.dryRun);
    });

    if (!registerTokens) {
        throw new Error('Error registering tokens');
    }
}

async function modifyTokenSupplyInFile(client: ClientManager, config: ConfigManager, options, _args, _fee) {
    const { env, tokenIds, chains } = options;
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;
    validateParameters({
        isNonEmptyString: { interchainTokenServiceAddress },
    });

    const tokens: SquidToken[] = await filteredTokens(env, tokenIds);

    const validateTokenSupplyResult = await forEachTokenAndChain(
        config,
        tokens,
        chains,
        async (token: SquidToken, chain: SquidTokenData) => {
            validateParameters({
                isNonEmptyString: {
                    tokenId: token.tokenId,
                    axelarChainId: chain.axelarChainId,
                    tokenAddress: chain.tokenAddress,
                },
            });
        },
    );

    const modifyTokenSupplyResult = await forEachTokenAndChain(config, tokens, chains, async (token: SquidToken, chain: SquidTokenData) => {
        const chainName = chain.axelarChainId.toLowerCase();
        await alignTokenSupplyOnHub(
            client,
            config,
            interchainTokenServiceAddress,
            token.tokenId,
            chain.tokenAddress,
            chainName,
            options.dryRun,
        );
    });

    if (!validateTokenSupplyResult || !modifyTokenSupplyResult) {
        throw new Error('Error validating or modifying token supply');
    }
}

const programHandler = () => {
    const program = new Command();

    program
        .name('ITS p2p token registration script for Squid and non-Squid config files')
        .description(
            'The script will register the P2P tokens to the ITS Hub or check if they are already registered.\n' +
                'Requires the token file to be present in the following path:\n' +
                ' * `axelar-chains-config/info/tokens-p2p/tokens-${env}.json`\n' +
                'The tokens file should follow the Squid config format.\n',
        );

    const registerTokensCmd = program
        .command('register-tokens')
        .description('Register tokens to the ITS Hub.')
        .addOption(new Option('-n, --chains <chains...>', 'chains to run the script for. Default: all chains'))
        .addOption(new Option('--tokenIds <tokenIds...>', 'tokenIds to run the script for. Default: all tokens'))
        .addOption(new Option('--dryRun', 'provide to just print out what will happen when running the command.'))
        .addOption(
            new Option('-m, --mnemonic <mnemonic>', 'Mnemonic of the InterchainTokenService operator account')
                .makeOptionMandatory(true)
                .env('MNEMONIC'),
        )
        .action((options) => {
            mainProcessor(registerTokensInFile, options, []);
        });

    addEnvOption(registerTokensCmd);

    const alignTokenSupplyCmd = program
        .command('align-token-supply')
        .description('Align the supply of a token on a chain with the supply on the chain.')
        .addOption(new Option('-n, --chains <chains...>', 'chains to run the script for. Default: all chains'))
        .addOption(new Option('--tokenIds <tokenIds...>', 'tokenIds to run the script for. Default: all tokens'))
        .addOption(new Option('--dryRun', 'provide to just print out what will happen when running the command.'))
        .addOption(
            new Option('-m, --mnemonic <mnemonic>', 'Mnemonic of the InterchainTokenService operator account')
                .makeOptionMandatory(true)
                .env('MNEMONIC'),
        )
        .action((options) => {
            mainProcessor(modifyTokenSupplyInFile, options, []);
        });

    addEnvOption(alignTokenSupplyCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
