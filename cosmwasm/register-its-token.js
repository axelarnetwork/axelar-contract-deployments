require('dotenv').config();

const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const { Command, Option } = require('commander');
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json');
const fs = require('fs');
const { printError, loadConfig, saveConfig, printInfo } = require('../common');
const { initContractConfig, prepareWallet, prepareClient } = require('./utils');
const { addAmplifierOptions } = require('./cli-utils');

class TokenIterator {
    constructor(env, info, tokenInfo, options) {
        this.env = env;
        this.info = info;
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
        return this.info.chains[this.chainName()].axelarId;
    }

    rpc() {
        if (this.rpcs) return this.rpcs[this.chainIndex];

        const chainName = this.chainName();
        return this.info.chains[chainName].rpc;
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

async function registerToken(client, wallet, tokenIterator, options) {
    const config = loadConfig(tokenIterator.env);

    initContractConfig(config, { contractName: 'InterchainTokenService' });

    const supply = tokenIterator.get().supply;
    const supplyParam = supply ? { tracked: String(supply) } : 'untracked';
    const msg = {
        register_p2p_token_instance: {
            chain: tokenIterator.axelarId(),
            token_id: tokenIterator.tokenId().slice(2),
            origin_chain: tokenIterator.info.chains[tokenIterator.token().originChain].axelarId,
            decimals: tokenIterator.get().decimals,
            supply: supplyParam,
        },
    };

    const interchainTokenServiceAddress = tokenIterator.info.axelar.contracts.InterchainTokenService.address;
    const registered = await client.queryContractSmart(interchainTokenServiceAddress, {
        token_instance: { chain: tokenIterator.chainName(), token_id: tokenIterator.tokenId().slice(2) },
    });
    if (registered) return;

    const [account] = await wallet.getAccounts(interchainTokenServiceAddress, {});
    printInfo('Registerring ', msg.register_p2p_token_instance);
    if (!options.dryRun) {
        await client.execute(account.address, interchainTokenServiceAddress, msg, 'auto');
        // If registration is successfull skip this token in the future without needing to query.
        tokenIterator.get().registered = true;
    }

    saveConfig(config, options.env);
}

const processCommand = async (options) => {
    const { env } = options;
    const info = require(`../axelar-chains-config/info/${env}.json`);
    const tokenInfo = require(`../axelar-chains-config/info/tokens-${env}.json`);
    const config = loadConfig(env);

    if (options.rpcs && (!options.chains || options.chains.length != options.rpcs.length)) {
        throw new Error('Need to provide chain names alongside RPCs and their length must match.');
    }

    initContractConfig(config, options);

    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);

    let iter = new TokenIterator(env, info, tokenInfo, options);

    while (await iter.getNext()) {
        await registerToken(client, wallet, iter, options);
    }

    saveConfig(config, env);
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
        processCommand(options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
