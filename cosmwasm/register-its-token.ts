import IInterchainToken from '@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json';
import { Command, Option } from 'commander';
import 'dotenv/config';
import { Contract, getDefaultProvider } from 'ethers';
import fs from 'fs';

import { printError, printInfo } from '../common';
import { ConfigManager } from '../common/config';
import { mainProcessor } from './processor';

// class TokenIterator {
//     env: string;
//     config: ConfigManager;
//     tokenInfo: any;
//     tokenIndex: number;
//     tokenIds: string[];
//     chains: string[];
//     chainIndex: number;
//     chainNames: string[];

//     constructor(env: string, config: ConfigManager, tokenInfo: any, options: any) {
//         this.env = env;
//         this.config = config;
//         this.tokenInfo = tokenInfo;
//         this.tokenIndex = -1;
//         this.tokenIds = options.tokenIds || Object.keys(tokenInfo);
//         this.chains = options.chains;
//         if (!this.incrementTokenIndex()) throw new Error('No tokens found matching the provided params.');
//         this.chainIndex = -1;
//     }

//     token() {
//         return this.tokenInfo[this.tokenIds[this.tokenIndex]];
//     }

//     incrementTokenIndex() {
//         while (true) {
//             if (this.tokenIndex >= this.tokenIds.length - 1) return false;

//             this.tokenIndex++;
//             const token = this.token();
//             if (!token) continue;

//             this.chainNames = this.chains || Object.keys(token).slice(0, -1);
//             this.chainIndex = 0;
//             return true;
//         }
//     }

//     get() {
//         return this.token()[this.chainName()];
//     }

//     chainName() {
//         return this.chainNames[this.chainIndex];
//     }

//     rpc() {
//         const rpcsFromTokenInfo = this.tokenInfo.chains[this.chainName()].rpcs;
//         if (rpcsFromTokenInfo && rpcsFromTokenInfo.length > 0) {
//             return rpcsFromTokenInfo[0];
//         }

//         return this.config.chains[this.chainName()].rpc;
//     }

//     async getNext() {
//         const previous = this.get();
//         if (previous && previous.supply) delete previous.supply;
//         fs.writeFileSync(`./axelar-chains-config/info/tokens-${this.env}.json`, JSON.stringify(this.tokenInfo, null, 2));
//         while (true) {
//             this.chainIndex++;

//             if (this.chainIndex >= this.chainNames.length) {
//                 if (!this.incrementTokenIndex()) return false;
//             }

//             const chainName = this.chainName();

//             const token = this.token();
//             if (chainName === token.originChain.origin_chain) continue;

//             const current = this.get();

//             if (!current) continue;

//             if (!current.registered) {
//                 if (current.track)
//                     try {
//                         const provider = getDefaultProvider(this.rpc());
//                         const token = new Contract(current.tokenAddress, IInterchainToken.abi, provider);
//                         current.supply = await token.totalSupply();
//                     } catch (e) {
//                         printError('Failed to query token supply for', current.tokenAddress);
//                     }

//                 printInfo(
//                     `Chain Progress: ${this.chainIndex + 1}/${this.chainNames.length} | Token Progress: ${this.tokenIndex + 1}/${this.tokenIds.length}`,
//                 );
//                 return true;
//             }
//         }
//     }
// }

async function getSupply(tokenAddress, rpc) {
    const provider = getDefaultProvider(rpc);
    const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
    return await token.totalSupply();
}

async function getOriginChain(tokenData, client, itsAddress: string) {
    // if only a single token exists it has to be the origin token (those will be skipped later).
    if (tokenData.chains.length === 1) {
        return tokenData.chains[0];
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
    const untracked = tokenData.chains.filter((chain) => !chain.tokenInfo?.track);
    if (untracked.length === 1) {
        printInfo(`Untracked token ${tokenData.tokenId} on ${untracked[0].chainName}`);
        return untracked[0].chainName;
    }

    // just use the first chain that shows up.
    return tokenData.chains[0];
}

type TokenDataToRegister = {
    tokenId: string;
    originChain: string;
    decimals: number;
    track: boolean;
    supply: string;
    axelarId: string;
};

async function registerToken(config: ConfigManager, client, tokenDataToRegister: TokenDataToRegister, dryRun: boolean) {
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
    if (!dryRun) {
        await client.execute(account.address, interchainTokenServiceAddress, msg, 'auto');

        // TODO tkulik: better to query the chain for the token registration?
        // If registration is successfull skip this token in the future without needing to query.
        // tokenIterator.get().registered = true;
    }
}

async function processCommand(client, config, options, _args, _fee) {
    const { env } = options;
    const tokenInfoString = fs.readFileSync(`../axelar-chains-config/info/tokens-p2p/tokens-${env}.json`, 'utf8');
    const tokenInfo = JSON.parse(tokenInfoString);

    tokenInfo.tokens.map(async (tokenData) => {
        tokenData.chains
            .filter((chain) => chain.tokenInfo?.track)
            .map(async (tokenOnChain): Promise<TokenDataToRegister> => {
                const chainName = Object.keys(tokenOnChain)[0];
                return {
                    tokenId: tokenData.tokenId,
                    originChain: chainName,
                    decimals: tokenOnChain.tokenInfo?.decimals,
                    track: tokenOnChain.tokenInfo?.track,
                    supply: await getSupply(tokenOnChain.tokenAddress, config.chains[chainName].rpc),
                    axelarId: config.chains[chainName].axelarId,
                };
            });
    });
}

const programHandler = () => {
    const program = new Command();

    program.name('register-its-token').description('Register tokens to the ITS Hub.');

    // TODO tkulik: Should we run this script for all chains per tokenID at once?
    program.addOption(new Option('-chains, --chains <chains...>', 'chains to run the script for').env('CHAINS'));
    program.addOption(new Option('-tokenIds, --tokenIds <tokenIds...>', 'tokenIds to run the script for').env('TOKEN_IDS'));
    program.addOption(new Option('-dryRun, --dryRun', 'provide to just print out what will happen when running the command.'));
    program.addOption(
        new Option('-m, --mnemonic <mnemonic>', 'Mnemonic of the InterchainTokenService account').makeOptionMandatory(true).env('MNEMONIC'),
    );

    // TODO tkulik: Do we need to add a flag to skip the prompt confirmation?
    program.addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'));

    program.action((options) => {
        mainProcessor(processCommand, options, []);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
