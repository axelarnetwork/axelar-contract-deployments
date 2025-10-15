const { copyMovePackage, TxBuilder, SUI_PACKAGE_ID } = require('@axelar-network/axelar-cgp-sui');
const { findPublishedObject, getObjectIdsByObjectTypes, moveDir, getStructs } = require('./utils');
const { broadcastFromTxBuilder } = require('./sign-utils');

async function deployTokenFromInfo(config, symbol, name, decimals) {
    if (!name || !symbol || !decimals) throw new Error('Token name, symbol and decimals are required');

    // Define the interchain token options
    copyMovePackage('interchain_token', null, moveDir);
    const interchainTokenOptions = {
        symbol,
        name,
        decimals,
    };

    const txBuilder = new TxBuilder(config.client);

    // Token Capability
    const tokenCap = await txBuilder.publishInterchainToken(moveDir, interchainTokenOptions);
    txBuilder.tx.transferObjects([tokenCap], config.walletAddress);

    // Publish token and derive type
    const publishTxn = await broadcastFromTxBuilder(txBuilder, config.keypair, `Published ${symbol}`, config.options);
    const publishObject = findPublishedObject(publishTxn);
    const packageId = publishObject.packageId;

    const tokenType = `${packageId}::${symbol.toLowerCase()}::${symbol.toUpperCase()}`;
    const [treasuryCap, metadata] = getObjectIdsByObjectTypes(publishTxn, [`TreasuryCap<${tokenType}>`, `Metadata<${tokenType}>`]);

    return [metadata, packageId, tokenType, treasuryCap];
}

async function createLockedCoinManagement(config, itsConfig, tokenType) {
    const txBuilder = new TxBuilder(config.client);

    const coinManagement = await txBuilder.moveCall({
        target: `${itsConfig.address}::coin_management::new_locked`,
        typeArguments: [tokenType],
    });

    return [txBuilder, coinManagement];
}

async function saveTokenDeployment(
    address, // package id
    tokenType, // coin type <T>
    contracts, // contracts object (from json config)
    symbol, // token symbol
    decimals, // token decimals
    TokenId, // ITS token id
    TreasuryCap, // sui::coin::TreasuryCap
    Metadata, // sui::coin::CoinMetadata
    linkedTokens = [], // [{chain, address, linkParams}]
    saltAddress = null, // address used for Bytes32::new for custom coin registrations and link_coin
    tokenManagerType = null,
) {
    contracts[symbol.toUpperCase()] = {
        address,
        typeArgument: tokenType,
        decimals,
        objects: {
            TokenId,
            TreasuryCap,
            Metadata,
        },
    };
    if (linkedTokens.length) contracts[symbol.toUpperCase()].linkedTokens = linkedTokens;
    if (saltAddress) contracts[symbol.toUpperCase()].saltAddress = saltAddress;
    if (tokenManagerType) contracts[symbol.toUpperCase()].tokenManagerType = tokenManagerType;
}

async function checkIfCoinExists(client, coinPackageId, coinType) {
    const structs = await getStructs(client, coinPackageId);

    if (!Object.values(structs).includes(coinType)) {
        throw new Error(`Coin type ${coinType} does not exist in package ${coinPackageId}`);
    }
}

/**
 * Get a coin object id for a coin held by the user and meeting a given threshold.
 * Returns `undefined` if balance threshold criteria are not met.
 * @param {Object} client : Sui client
 * @param {String} walletAddress : Sui wallet address
 * @param {String} coinType : Named type of the Sui coin
 * @param {String | Number} : Balance threshold required, in unit amount format (@see getUnitAmount)
 * @returns Coin Object ID held by user which has sufficient balance
 */
async function senderHasSufficientBalance(client, keypair, coinType, amount) {
    const walletAddress = keypair.toSuiAddress();

    const coins = await client.getCoins({
        owner: walletAddress,
        coinType,
    });

    const insufficientBalanceMsg = `Insufficient balance of coin ${coinType} using wallet ${walletAddress}`;
    if (!Array.isArray(coins.data) || !coins.data.length) {
        throw new Error(insufficientBalanceMsg);
    }

    let coin = coins.data.find((c) => parseInt(c.balance) >= parseInt(amount));

    // Merge coins to reach required threshold if possible
    if (!coin) {
        const txBuilder = new TxBuilder(client);

        let totalBalance = 0;
        coins.data.forEach((coin) => {
            const { balance } = coin;
            totalBalance += parseInt(balance);
        });

        if (totalBalance < parseInt(amount)) {
            throw new Error(insufficientBalanceMsg);
        }

        const coinObjectIds = coins.data.map((coin) => coin.coinObjectId);
        const firstCoin = coinObjectIds.shift();
        const remainingCoins = coinObjectIds.map((id) => tx.object(id));

        txBuilder.tx.mergeCoins(firstCoin, remainingCoins);

        await broadcastFromTxBuilder(txBuilder, keypair, 'Merge coins', options);

        coin = coins.data.find((c) => c.coinObjectId === firstCoin);
    }

    return coin;
}

module.exports = {
    deployTokenFromInfo,
    createLockedCoinManagement,
    saveTokenDeployment,
    checkIfCoinExists,
    senderHasSufficientBalance,
};
