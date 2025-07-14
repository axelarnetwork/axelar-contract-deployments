const { copyMovePackage, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { findPublishedObject, getObjectIdsByObjectTypes, moveDir } = require('./utils');
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

async function newCoinManagementLocked(config, itsConfig, tokenType) {
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
    if (linkedTokens.length) contracts[symbol.toUpperCase()] = linkedTokens;
}

module.exports = {
    deployTokenFromInfo,
    newCoinManagementLocked,
    saveTokenDeployment,
};
