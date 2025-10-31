require('dotenv').config();
const env = process.env.ENV;

const { ethers } = require('hardhat');
const {
    Contract,
    getDefaultProvider,
    constants: { AddressZero },
} = ethers;
const info = require(`../axelar-chains-config/info/${env}.json`);
const tokenManagerInfo = require(`../axelar-chains-config/info/tokenManagers-${env}.json`);
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const IInterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainToken.sol/IInterchainToken.json');
const fs = require('fs');
const { printInfo } = require('../common');

const RPCs = require(`../axelar-chains-config/rpcs/${env}.json`);

async function getTokens(name) {
    try {
        const chain = info.chains[name];
        if (
            !chain.contracts.InterchainTokenService ||
            chain.contracts.InterchainTokenService.skip ||
            !tokenManagerInfo[name] ||
            !tokenManagerInfo[name].tokenManagers
        )
            return false;
        printInfo(`ITS at ${name} is at`, chain.contracts.InterchainTokenService.address);
        printInfo('processing... ', name);

        const rpc = RPCs[name];
        const provider = getDefaultProvider(rpc);
        const tokenManagers = tokenManagerInfo[name].tokenManagers;

        let i = 0;
        let tries = 0;
        while (i < tokenManagers.length) {
            try {
                const tokenData = tokenManagers[i];
                if (tokenData.decimals != null) {
                    i++;
                    continue;
                }
                const tokenId = tokenData.tokenId;
                const tokenManagerAddress = tokenData.tokenManagerAddress;
                const tokenManagerType = tokenData.tokenManagerType;

                printInfo(`${name}: Processing (${i}/${tokenManagers.length}), tokenId: ${tokenId}`);
                const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
                const tokenAddress = await tokenManager.tokenAddress();
                const token = new Contract(tokenAddress, IInterchainToken.abi, provider);
                const tokenDecimals = await token.decimals();
                const decimals = tokenDecimals;
                const track = tokenManagerType === 0 && (await token.isMinter(AddressZero));
                tokenData.tokenAddress = tokenAddress;
                tokenData.decimals = decimals;
                tokenData.track = track;
                fs.writeFileSync(`./axelar-chains-config/info/tokenManagers-${env}.json`, JSON.stringify(tokenManagerInfo, null, 2));
                tries = 0;
                i++;
            } catch (e) {
                tries++;
                if (tries >= 10) {
                    printError(`Error getting tokens for ${name}: ${e.message}`);
                    i++;
                }
            }
        }
        return true;
    } catch (e) {
        printError(`Error getting tokens for ${name}: ${e.message}`);
    }
}

(async () => {
    for (const name of Object.keys(info.chains)) {
        getTokens(name).then((success) => printInfo(name, 'returned', success));
    }
})();
