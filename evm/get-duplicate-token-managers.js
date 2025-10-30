
require('dotenv').config();
const env = process.env.ENV;
const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const fs = require('fs');
const { printInfo } = require('../common');

const RPCs = require(`../axelar-chains-config/rpcs/${env}.json`);

async function getTokens(name) {
    const chainName = name.toLowerCase();

    if (!tokenManagerInfo[name]) return;
    const tokenManagers = tokenManagerInfo[name].tokenManagers;
    const chain = info.chains[name];
    if (chain.contracts.InterchainTokenService.skip) return;
    const rpc = RPCs[chain];
    const provider = getDefaultProvider(rpc);
    const service = new Contract(chain.contracts.InterchainTokenService.address, IInterchainTokenService.abi, provider);

    let counter = 0;
    let index = 1;
    const finalResult = [];

    for (const tokenData of tokenManagers) {
        // event TokenManagerDeployed(tokenId, tokenManager_, tokenManagerType, params);
        const tokenId = tokenData[0];
        const tokenManagerAddress = tokenData[1];
        const tokenManagerType = tokenData[2];
        const interchainTokenAddress = await service.interchainTokenAddress(tokenId);

        printInfo('Processing (%s/%s), tokenId: %s', index, tokenManagers.length, tokenId);
        const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
        tokenData.tokenAddress = await tokenManager.tokenAddress();

        if (interchainTokenAddress !== tokenData.tokenAddress && tokenManagerType === 0) {
            const result = {};
            result.tokenId = tokenId;
            result.tokenManager_tokenAddress = tokenData.tokenAddress;
            result.interchainTokenAddress = interchainTokenAddress;
            result.tokenManagerType = tokenManagerType;

            printInfo(result);
            finalResult.push(result);
            counter++;
        }

        index++;
    }

    fs.writeFileSync(`result_${chainName}.json`, JSON.stringify(finalResult, null, 2));
    printInfo('Chain: %s, Diff Address: %s out of %s', chainName, counter, tokenManagers.length);
}

(async () => {
    for (const name of Object.keys(info.chains)) {
        printInfo('Chain Name', name);

        try {
            await getTokens(name);
        } catch (e) {
            printError(`Error getting tokens for ${name}: ${e.message}`);
        }
    }
})();
