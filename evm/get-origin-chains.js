require('dotenv').config();
const env = process.env.ENV;

const { ethers } = require('hardhat');
const { Contract, getDefaultProvider, constants: { AddressZero }, utils: {arrayify} } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json')
const fs = require('fs');
const toml = require('toml');
const { printInfo, loadConfig } = require('../common');
const { initContractConfig, prepareWallet, prepareClient, instantiateContract } = require('../cosmwasm/utils');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

//const RPCs = toml.parse(fs.readFileSync(`./axelar-chains-config/info/rpcs-${env}.toml`, 'utf-8'));
//const squidTokens = require(`../axelar-chains-config/info/squid-tokens-${env}.json`);

(async () => {
    const tokens = {};
    for (const name of Object.keys(tokenManagerInfo)) {
        for (const token of tokenManagerInfo[name].tokenManagers) {
            if(!token.tokenId) {
                continue;
            }
            if (! tokens[token.tokenId] ) tokens[token.tokenId] = {};

            tokens[token.tokenId][name] = token;
        }
    }
    for (const i in Object.keys(tokens)) {
        const tokenId = Object.keys(tokens)[i];
        console.log(`${Number(i) + 1} / ${Object.keys(tokens).length}: ${tokenId}`)
        const token = tokens[tokenId];
        const chainNames = Object.keys(token);

        // if only a single token exists it has to be the origin token (those will be skipped later).
        if (chainNames.length === 1) {
            token.originChain = chainNames[0];
            continue;
        }

        // if a token is already registered on axelar, use the same origin chain.
        const client = await CosmWasmClient.connect(info.axelar.rpc);

        try {
            const originChain = await client.queryContractSmart(info.axelar.contracts.InterchainTokenService.address, {
                "token_config": {"token_id": tokenId.slice(2)}
            });
            if(originChain) {
                token.originChain = originChain.origin_chain;
                continue;
            }
        } catch (e) {
            console.log(e);
        }

        // if squid is tracking this token, use this value.
        /*if (squidTokens.tokens[tokenId]) {
            token.originChain = squidTokens.tokens[tokenId].originAxelarChainId;
            continue;
        }*/

        // if only a single chain is untacked, use that chain
        const untracked = [];
        for(const chainName of Object.keys(token)) {
            if(!token[chainName].tracking) {
                untracked.push(chainName);
            }
        }
        if (untracked.length === 1) {
            console.log(untracked[0]);
            token.originChain = untracked[0];
            continue;
        }

        // just use the firt chain that shows up.
        token.originChain = chainNames[0];
    }
    fs.writeFileSync(`./axelar-chains-config/info/tokens-${env}.json`, JSON.stringify(tokens, null, 2));
})();
