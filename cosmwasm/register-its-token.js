require('dotenv').config();

const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const { Command, Option } = require('commander');
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json')
const fs = require('fs');
const { printError, loadConfig, saveConfig } = require('../common');
const { initContractConfig, prepareWallet, prepareClient } = require('./utils');
const { addAmplifierOptions } = require('./cli-utils');

class TokenIterator {
    constructor(env, info, tokenInfo, options) {
        console.log(options);
        this.env = env
        this.info = info;
        this.tokenInfo = tokenInfo;
        this.tokenIndex = -1;
        this.tokenIds = options.tokenIds || Object.keys(tokenInfo);
        this.chains = options.chains;
        this.rpcs = options.rpcs;
        if(!this.incrementTokenIndex()) throw new Error('No tokens found matching the provided params.');
        this.chainIndex = -1;
    }

    tokenId() {
        return this.tokenIds[this.tokenIndex];
    }

    token() {
        return this.tokenInfo[this.tokenId()];
    }

    incrementTokenIndex() {
        while(true) {
            if(this.tokenIndex >= this.tokenIds.length - 1) return false;
        
            this.tokenIndex++;
            const token = this.token();
            if(!token) continue;
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

    rpc() {
        if(this.rpcs) return this.rpcs[this.chainIndex];

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
                if(!this.incrementTokenIndex()) return false;
            }
            const chainName = this.chainName();

            const token = this.token();
            if(chainName === token.originChain.origin_chain) continue;

            const current = this.get();
            
            if (!current) continue;

            if (!current.registered) {
                if (current.track) try {
                    const provider = getDefaultProvider(this.rpc());
                    const token = new Contract(current.tokenAddress, IInterchainToken.abi, provider);
                    current.supply = await token.totalSupply();
                } catch (e) {
                    printError('Failed to query token supply for', current.tokenAddress);
                }
                console.log(`Chain Progress: ${this.chainIndex + 1}/${this.chainNames.length} | Token Progress: ${this.tokenIndex + 1}/${this.tokenIds.length}`)
                return true;
            }
        }
    }
}

async function registerToken(client, wallet, tokenIterator) {
    const config = loadConfig(tokenIterator.env);

    initContractConfig(config, {contractName: "InterchainTokenService"});

    const supply = tokenIterator.get().supply;
    const supplyParam = supply ? {"tracked": String(supply)} : "untracked";
    const msg = { "register_p2p_token_instance": {
        "chain": tokenIterator.chainName(),
        "token_id": tokenIterator.tokenId().slice(2),
        "origin_chain": tokenIterator.token().originChain,
        "decimals": tokenIterator.get().decimals,
        "supply": supplyParam,
    } };
    console.log(msg); return;
    const registeredChains = {};
    const registered = await client.queryContractSmart(info.axelar.contracts.InterchainTokenService.address, {"token_instance": {chain: tokenIterator.chainName(), "token_id": tokenIterator.tokenId().slice(2)}});
    if (registered) return;
    const [account] = await wallet.getAccounts(info.axelar.contracts.InterchainTokenService.address, {});

    const ensureChainRegistered = async (chain) => {
        if(registeredChains[chain]) return;
        registeredChains[chain] = true;
        if(await client.queryContractSmart(info.axelar.contracts.InterchainTokenService.address, {"its_chain": {"chain": chain}})) return;
        const msg = {"register_chains":{"chains":[{"chain":chain,"its_edge_contract":"source-its-contract","truncation":{"max_uint_bits":256,"max_decimals_when_truncating":255}}]}};
        await client.execute(account.address, info.axelar.contracts.InterchainTokenService.address, msg, 'auto');
    }
    await ensureChainRegistered(tokenIterator.chainName());
    await ensureChainRegistered(tokenIterator.token().originChain);
    
    await client.execute(account.address, info.axelar.contracts.InterchainTokenService.address, msg, 'auto');
    // If registration is successfull skip this token in the future without needing to query.
    tokenIterator.get().registered = true;

    saveConfig(config, env);
}

const processCommand = async (options) => {
    const { env } = options;
    const info = require(`../axelar-chains-config/info/${env}.json`);
    const tokenInfo = require(`../axelar-chains-config/info/tokens-${env}.json`);
    const config = loadConfig(env);

    if (options.rpcs && (!options.chains || options.chains.length != options.rpcs.length)) throw new Error('Need to provide chain names alongside RPCs and their length must match.');

    initContractConfig(config, options);

    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);

    let iter = new TokenIterator(env, info, tokenInfo, options);

    while (await iter.getNext()) {
        await registerToken(client, wallet, iter);
    }

    saveConfig(config, env);
};

const programHandler = () => {
    const program = new Command();

    program.name('register-its-token').description('Register tokens to the ITS Hub.');

    addAmplifierOptions(program, {});

    program.addOption(new Option('-tokenIds, --tokenIds <tokenIds...>', 'tokenIds to run the script for'));
    program.addOption(new Option('-chains, --chains <chains...>', 'chains to run the script for'));
    program.addOption(new Option('-rpcs, --rpcs <rpcs...>', 'rpcs. Must be provided alongside a --chains argument and their length must match'));

    program.action((options) => {
        processCommand(options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
