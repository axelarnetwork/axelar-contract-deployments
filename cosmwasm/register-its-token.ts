import 'dotenv/config';

import { Contract, getDefaultProvider } from 'ethers';
import { Command, Option } from 'commander';
import IInterchainToken from '@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json';
import fs from 'fs';
import { printError, printInfo } from '../common';
import { addAmplifierOptions } from './cli-utils';
import { mainProcessor } from './processor';
import { ConfigManager } from '../common/config';

class TokenIterator {

    env: string;
    config: ConfigManager;
    tokenInfo: any;
    tokenIndex: number;
    tokenIds: string[];
    chains: string[];
    rpcs: string[];
    chainIndex: number;
    chainNames: string[];


    constructor(env: string, config: ConfigManager, tokenInfo: any, options: any) {
        this.env = env;
        this.config = config;
        this.tokenInfo = tokenInfo;
        this.tokenIndex = -1;
        this.tokenIds = options.tokenIds || Object.keys(tokenInfo);
        this.chains = options.chains;
        this.rpcs = options.rpcs;
        if (!this.incrementTokenIndex()) throw new Error('No tokens found matching the provided params.');
        this.chainIndex = -1;
    }

    tokenId() {
        return this.tokenIds[this.tokenIndex];
    }

    token() {
        return this.tokenInfo[this.tokenId()];
    }

    incrementTokenIndex() {
        while (true) {
            if (this.tokenIndex >= this.tokenIds.length - 1) return false;

            this.tokenIndex++;
            const token = this.token();
            if (!token) continue;

            this.chainNames = this.chains || Object.keys(token).slice(0, -1);
            this.chainIndex = 0;
            return true;
        }
    }

    get() {
        return this.token()[this.chainName()];
    }

    chainName() {
        return this.chainNames[this.chainIndex];
    }

    axelarId() {
        return this.config.chains[this.chainName()].axelarId;
    }

    rpc() {
        if (this.rpcs) return this.rpcs[this.chainIndex];

        const chainName = this.chainName();
        return this.config.chains[chainName].rpc;
    }

    async getNext() {
        const previous = this.get();
        if (previous && previous.supply) delete previous.supply;
        fs.writeFileSync(`./axelar-chains-config/info/tokens-${this.env}.json`, JSON.stringify(this.tokenInfo, null, 2));
        while (true) {
            this.chainIndex++;

            if (this.chainIndex >= this.chainNames.length) {
                if (!this.incrementTokenIndex()) return false;
            }

            const chainName = this.chainName();

            const token = this.token();
            if (chainName === token.originChain.origin_chain) continue;

            const current = this.get();

            if (!current) continue;

            if (!current.registered) {
                if (current.track)
                    try {
                        const provider = getDefaultProvider(this.rpc());
                        const token = new Contract(current.tokenAddress, IInterchainToken.abi, provider);
                        current.supply = await token.totalSupply();
                    } catch (e) {
                        printError('Failed to query token supply for', current.tokenAddress);
                    }

                printInfo(
                    `Chain Progress: ${this.chainIndex + 1}/${this.chainNames.length} | Token Progress: ${this.tokenIndex + 1}/${this.tokenIds.length}`,
                );
                return true;
            }
        }
    }
}

async function getOriginChain(tokenData, client, itsAddress: string) {
    // if only a single token exists it has to be the origin token (those will be skipped later).
    if (tokenData.chains.length === 1) {
        return tokenData.chains[0];
    }

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
    const untracked = [];
    for (const chainName of tokenData.chains) {
        if (!tokenData.chains[chainName].tracking) {
            untracked.push(chainName);
        }
    }
    if (untracked.length === 1) {
        printInfo(`Untracked token ${tokenData.tokenId} on ${untracked[0]}`);
        return untracked[0];
    }

    // just use the firt chain that shows up.
    return tokenData.chains[0];
}

async function registerToken(client, config, tokenIterator, options) {
    const supply = tokenIterator.get().supply;
    const supplyParam = supply ? { tracked: String(supply) } : 'untracked';
    const msg = {
        register_p2p_token_instance: {
            chain: tokenIterator.axelarId(),
            token_id: tokenIterator.tokenId().slice(2),
            origin_chain: tokenIterator.config.chains[tokenIterator.token().originChain].axelarId,
            decimals: tokenIterator.get().decimals,
            supply: supplyParam,
        },
    };

    const interchainTokenServiceAddress = tokenIterator.config.getContractAddress('InterchainTokenService');
    const registered = await client.queryContractSmart(interchainTokenServiceAddress, {
        token_instance: { chain: tokenIterator.chainName(), token_id: tokenIterator.tokenId().slice(2) },
    });
    if (registered) return;

    // TODO tkulik: only InterchainTokenService can be account is used here.
    const [account] = await client.accounts;
    printInfo('Registerring ', JSON.stringify(msg.register_p2p_token_instance));
    if (!options.dryRun) {
        await client.execute(account.address, interchainTokenServiceAddress, msg, 'auto');
        // If registration is successfull skip this token in the future without needing to query.
        tokenIterator.get().registered = true;
    }
}

async function processCommand(client, config, options, _args, _fee) {
    const { env } = options;
    const tokenInfo = (await import(`../axelar-chains-config/info/tokens-${env}.json`)).default;

    if (options.rpcs && (!options.chains || options.chains.length != options.rpcs.length)) {
        throw new Error('Need to provide chain names alongside RPCs and their length must match.');
    }

    const iter = new TokenIterator(env, config, tokenInfo, options);

    while (await iter.getNext()) {
        await registerToken(client, config, iter, options);
    }
};

const programHandler = () => {
    const program = new Command();

    program.name('register-its-token').description('Register tokens to the ITS Hub.');

    addAmplifierOptions(program, {});

    program.addOption(new Option('-tokenIds, --tokenIds <tokenIds...>', 'tokenIds to run the script for').env('TOKEN_IDS'));

    // TODO tkulik: Should we run this script for all chains per tokenID at once?
    program.addOption(new Option('-chains, --chains <chains...>', 'chains to run the script for').env('CHAINS'));
    program.addOption(
        new Option('-rpcs, --rpcs <rpcs...>', 'rpcs. Must be provided alongside a --chains argument and their length must match').env(
            'RPCS',
        ),
    );
    program.addOption(new Option('-dryRun, --dryRun', 'provide to just print out what will happen when running the command.'));

    program.action((options) => {
        mainProcessor(processCommand, options, []);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
