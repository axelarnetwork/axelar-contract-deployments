require('dotenv').config();
const env = process.env.ENV;
const mnemonic = process.env.MNEMONIC;

const { ethers } = require('hardhat');
const { Contract, getDefaultProvider, constants: { AddressZero } } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenInfo = require(`../axelar-chains-config/info/tokens-${env}.json`);
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json')
const fs = require('fs');
const toml = require('toml');
const { printInfo, printError, loadConfig, saveConfig } = require('../common');
const { initContractConfig, prepareWallet, prepareClient, instantiateContract } = require('./utils');
const { GasPrice } = require('@cosmjs/stargate');

//const RPCs = toml.parse(fs.readFileSync(`./axelar-chains-config/info/rpcs-${env}.toml`, 'utf-8'));


class TokenIterator {
    constructor() {
        this.tokenIndex = -1;
        this.tokenIds = Object.keys(tokenInfo);
        this.incrementTokenIndex();
        this.chainIndex = -1;
    }

    tokenId() {
        return this.tokenIds[this.tokenIndex];
    }

    token() {
        return tokenInfo[this.tokenId()];
    }

    incrementTokenIndex() {
        if(this.tokenIndex >= this.tokenIds.length - 1) return false;
        this.tokenIndex++;
        this.chainNames = Object.keys(this.token()).slice(0, -1);
        this.chainIndex = 0;
        return true;
    }

    get() {
        return this.token()[this.chainName()];
    }

    chainName() {
        return this.chainNames[this.chainIndex];
    }

    rpc() {
        const chainName = this.chainName();
        return env === 'mainnet' || env === 'testnet' ? RPCs.axelar_bridge_evm.find((chain) => chain.name.toLowerCase() === chainName).rpc_addr : info.chains[chainName].rpc;
    }

    async getNext() {
        const previous = this.get();
        if (previous && previous.supply) delete previous.supply;
        fs.writeFileSync(`./axelar-chains-config/info/tokens-${env}.json`, JSON.stringify(tokenInfo, null, 2));                 
        while (true) {
            this.chainIndex++;
            if (this.chainIndex >= this.chainNames.length) {
                if(!this.incrementTokenIndex()) return false;
            }
            const chainName = this.chainName()
            if(chainName === this.token().originChain.origin_chain) continue;
            const current = this.get();
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

async function registerToken(tokenIterator) {
    const config = loadConfig(env);

    initContractConfig(config, {contractName: "InterchainTokenService"});

    const wallet = await prepareWallet({mnemonic});
    const client = await prepareClient(config, wallet);
    const supply = tokenIterator.get().supply;
    const supplyParam = supply ? {"tracked": String(supply)} : "untracked";
    const msg = { "register_p2p_token_instance": {
        "chain": tokenIterator.chainName(),
        "token_id": tokenIterator.tokenId().slice(2),
        "origin_chain": tokenIterator.token().originChain,
        "decimals": tokenIterator.get().decimals,
        "supply": supplyParam,
    } };
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

if (require.main === module) {
    (async () => {
        let iter = new TokenIterator();
        await iter.getNext();await iter.getNext();
        while(await iter.getNext()) {
            await registerToken(iter);
        }
    })();
}