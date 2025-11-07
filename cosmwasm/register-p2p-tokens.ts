import { Command, Option } from 'commander';
import fs from 'fs';

import { printError, printInfo } from '../common';
import { ConfigManager } from '../common/config';
import { isConsensusChain } from '../evm/utils';
import { TokenData, registerToken } from './its';
import { ClientManager, mainProcessor } from './processor';

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

function getOriginChain(tokenData: SquidToken, originChainName?: string) {
    if (originChainName) {
        return originChainName;
    }

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
    if (tokenData.chains[0]?.axelarChainId) {
        return tokenData.chains[0].axelarChainId;
    } else {
        throw new Error('No chains found for token ' + tokenData.tokenId);
    }
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

    const filteredTokens: SquidToken[] = Object.values(tokenInfo.tokens).filter(
        (token: SquidToken) => (tokenIds ? tokenIdsToProcess.has(token.tokenId) : true) && token.tokenType === 'interchain',
    );

    for (const token of filteredTokens) {
        const filteredChains: SquidTokenData[] = token.chains.filter((chain: SquidTokenData) => {
            try {
                return (
                    (chains ? chainsToProcess.has(chain.axelarChainId.toLowerCase()) : true) &&
                    chain.axelarChainId.toLowerCase() !== token.originAxelarChainId?.toLowerCase() &&
                    isConsensusChain(config.getChainConfig(chain.axelarChainId.toLowerCase()))
                );
            } catch (e) {
                printError(`Error getting chain config for ${chain.axelarChainId} (skipping chain): ${e}`);
                return false;
            }
        });

        for (const chain of filteredChains) {
            await processToken(token, chain);
        }
    }
}

async function registerTokensInFile(client: ClientManager, config: ConfigManager, options, _args, _fee) {
    const interchainTokenServiceAddress = config.getContractConfig('InterchainTokenService').address;

    await forEachTokenInFile(config, options, async (token: SquidToken, chain: SquidTokenData) => {
        try {
            const tokenDataToRegister: TokenData = {
                tokenId: token.tokenId,
                originChain: getOriginChain(token, token.originAxelarChainId),
                decimals: token.decimals,
                chainName: chain.axelarChainId.toLowerCase(),
            } as TokenData;
            await registerToken(config, interchainTokenServiceAddress, client, tokenDataToRegister, options.dryRun);
        } catch (e) {
            printError(`Error registering token ${token.tokenId} on ${chain.axelarChainId}: ${e}`);
        }
    });
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

    program
        .command('register-tokens')
        .description('Register tokens to the ITS Hub.')
        .addOption(new Option('-n, --chains <chains...>', 'chains to run the script for. Default: all chains').env('CHAINS'))
        .addOption(new Option('--tokenIds <tokenIds...>', 'tokenIds to run the script for. Default: all tokens').env('TOKEN_IDS'))
        .addOption(new Option('--dryRun', 'provide to just print out what will happen when running the command.'))
        .addOption(new Option('--env <env>', 'environment to run the script for').env('ENV').makeOptionMandatory(true))
        .addOption(
            new Option('-m, --mnemonic <mnemonic>', 'Mnemonic of the InterchainTokenService operator account')
                .makeOptionMandatory(true)
                .env('MNEMONIC'),
        )
        .action((options) => {
            mainProcessor(registerTokensInFile, options, []);
        });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
