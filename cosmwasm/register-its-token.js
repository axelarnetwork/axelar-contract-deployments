require('dotenv').config();
const env = process.env.ENV;

const { ethers } = require('hardhat');
const { Contract, getDefaultProvider, constants: { AddressZero } } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json')
const fs = require('fs');
const toml = require('toml');
const { printInfo, printError } = require('../common');

const RPCs = toml.parse(fs.readFileSync('./axelar-chains-config/info/rpcs.toml', 'utf-8'));


class TokenIterator {
    constructor() {
        this.chainIndex = 0;
        this.tokenIndex = -1;
        this.chainKeys = Object.keys(tokenManagerInfo);
        this.provider = getDefaultProvider(this.rpc());
    }

    get() {
        return tokenManagerInfo[this.chainKeys[this.chainIndex]].tokenManagers[this.tokenIndex];
    }

    chain() {
        return info[this.chainKeys[this.chainIndex]];
    }

    rpc() {
        const name = this.chainKeys[this.chainIndex];
        const chain = info.chains[name];
        return env === 'mainnet' ? RPCs.axelar_bridge_evm.find((chain) => chain.name.toLowerCase() === name).rpc_addr : chain.rpc;
    }

    async getNext() {
        const previous = this.get();
        if (previous && previous.supply) delete previous.supply;
        fs.writeFileSync(`./axelar-chains-config/info/tokenManagers-${env}.json`, JSON.stringify(tokenManagerInfo, null, 2));                 
        while (true) {
            this.tokenIndex++;
            while (tokenManagerInfo[this.chainKeys[this.chainIndex]].tokenManagers.length <= this.tokenIndex) {
                this.chainIndex++;
                if (this.chainIndex >= this.chainKeys.length) return false;
                this.tokenIndex = 0;
                this.provider = getDefaultProvider(this.rpc());
            }
            const current = this.get();
            if (current.decimals && !current.registered) {
                if (current.track) try {
                    const token = new Contract(current.tokenAddress, IInterchainToken.abi, this.provider);
                    current.supply = await token.totalSupply();
                } catch (e) {
                    printError('Failed to query token supply for', current.tokenAddress);
                }
                console.log(`Chain Progress: ${this.chainIndex + 1}/${this.chainKeys.length} | Token Progress: ${this.tokenIndex + 1}/${tokenManagerInfo[this.chainKeys[this.chainIndex]].tokenManagers.length}`)
                return current;
            }
        }
    }
}

async function registerToken(token) {
    // TODO: register token here
    // If registration is successfull skip this token in the future without needing to query.
    token.registered = true;
}

if (require.main === module) {
    (async () => {
        let iter = new TokenIterator();
        let token = await iter.getNext();

        while(token) {
            await registerToken(token);
            token = await iter.getNext();
        }
    })();
}