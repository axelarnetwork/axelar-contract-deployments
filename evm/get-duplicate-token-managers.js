const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const info = require('../axelar-chains-config/info/mainnet.json');
const tokenManagerInfo = require('../axelar-chains-config/info/tokenManagers.json');
const ITokenManager = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/ITokenManager.sol/ITokenManager.json');
const IInterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
const fs = require('fs');
const toml = require('toml');

const RPCs = toml.parse(fs.readFileSync('./axelar-chains-config/info/rpcs.toml', 'utf-8'));
const itsAddresses = {
    ethereum: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    avalanche: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    fantom: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    polygon: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    moonbeam: '0xb5fb4be02232b1bba4dc8f81dc24c26980de9e3c',
    binance: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    arbitrum: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    celo: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    kava: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    filecoin: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    optimism: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    linea: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    base: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    mantle: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    blast: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    fraxtal: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
    scroll: '0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C',
};

async function getTokens(name) {
    const chainName = name.toLowerCase();

    /*
    if (chainName == 'filecoin') {
        console.log('skipping: ', name);
        return;
    } else if (chainName == 'scroll') {
        console.log('processing: ', name);
    } else {
        console.log('skipping: ', name);
        return;
    }*/

    if (!tokenManagerInfo[name]) return;
    const tokenManagers = tokenManagerInfo[name].tokenManagers;
    const chain = info.chains[name];
    if (chain.contracts.InterchainTokenService.skip) return;

    const rpc = RPCs.axelar_bridge_evm.find((chain) => chain.name.toLowerCase() === chainName).rpc_addr;
    const provider = getDefaultProvider(rpc);
    const service = new Contract(itsAddresses[chainName], IInterchainTokenService.abi, provider);

    let counter = 0;
    let index = 1;
    const finalResult = [];

    for (const tokenData of tokenManagers) {
        // event TokenManagerDeployed(tokenId, tokenManager_, tokenManagerType, params);
        const tokenId = tokenData[0];
        const tokenManagerAddress = tokenData[1];
        const tokenManagerType = tokenData[2];
        const interchainTokenAddress = await service.interchainTokenAddress(tokenId);

        console.log('Processing (%s/%s), tokenId: %s', index, tokenManagers.length, tokenId);
        const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, provider);
        tokenData.tokenAddress = await tokenManager.tokenAddress();

        if (interchainTokenAddress !== tokenData.tokenAddress && tokenManagerType === 0) {
            const result = {};
            result.tokenId = tokenId;
            result.tokenManager_tokenAddress = tokenData.tokenAddress;
            result.interchainTokenAddress = interchainTokenAddress;
            result.tokenManagerType = tokenManagerType;

            console.log(result);
            finalResult.push(result);
            counter++;
        }

        index++;
    }

    fs.writeFileSync(`result_${chainName}.json`, JSON.stringify(finalResult, null, 2));
    console.log('Chain: %s, Diff Address: %s out of %s', chainName, counter, tokenManagers.length);

    /*
    const managerCounts = {};
    const tokenCounts = {};
    tokenManagers.forEach(function (x) {
        console.log(x.tokenAddress);
        tokenCounts[x.tokenAddress] = (tokenCounts[x.tokenAddress] || 0) + 1;
    });
    tokenManagers.forEach(function (x) {
        managerCounts[x[1]] = (managerCounts[x[1]] || 0) + 1;
    });
    duplicated = tokenManagers.filter((x) => tokenCounts[x.tokenAddress] > managerCounts[x[1]]);
    console.log(duplicated);
    console.log(duplicated.length);

    const mintBurn = tokenManagers.filter((x) => x[2] == 2);
    console.log(`MintBurn: ${mintBurn.length}`);

    const mintBurnFrom = tokenManagers.filter((x) => x[2] == 1);
    console.log(`MintBurnFrom: ${mintBurnFrom.length}`);
    */
}

(async () => {
    for (const name of Object.keys(info.chains)) {
        console.log(name);

        try {
            await getTokens(name);
        } catch (e) {
            console.log(name);
            console.log(e);
        }
    }
})();
