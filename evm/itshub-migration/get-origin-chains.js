require('dotenv').config();
const env = process.env.ENV;

const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);
const fs = require('fs');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { printInfo, printError } = require('../../common/utils');

(async () => {
    const tokens = {};
    for (const name of Object.keys(tokenManagerInfo)) {
        for (const token of tokenManagerInfo[name].tokenManagers) {
            if (!token.tokenId) {
                continue;
            }
            if (!tokens[token.tokenId]) tokens[token.tokenId] = {};

            tokens[token.tokenId][name] = token;
        }
    }
    for (const i in Object.keys(tokens)) {
        const tokenId = Object.keys(tokens)[i];
        printInfo(`${Number(i) + 1} / ${Object.keys(tokens).length}: ${tokenId}`);
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
                token_config: { token_id: tokenId.slice(2) },
            });
            if (originChain) {
                token.originChain = originChain.origin_chain;
                continue;
            }
        } catch (e) {
            printError(`Error getting origin chain for ${tokenId}: ${e.message}`);
        }

        // if squid is tracking this token, use this value.
        /*if (squidTokens.tokens[tokenId]) {
            token.originChain = squidTokens.tokens[tokenId].originAxelarChainId;
            continue;
        }*/

        // if only a single chain is untacked, use that chain
        const untracked = [];
        for (const chainName of Object.keys(token)) {
            if (!token[chainName].tracking) {
                untracked.push(chainName);
            }
        }
        if (untracked.length === 1) {
            printInfo(`Untracked token ${tokenId} on ${untracked[0]}`);
            token.originChain = untracked[0];
            continue;
        }

        // just use the firt chain that shows up.
        token.originChain = chainNames[0];
    }
    fs.writeFileSync(`./axelar-chains-config/info/tokens-${env}.json`, JSON.stringify(tokens, null, 2));
})();
