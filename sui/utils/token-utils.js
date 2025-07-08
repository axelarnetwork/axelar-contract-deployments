const { Transaction } = require('@mysten/sui/transactions');
const { copyMovePackage, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { findPublishedObject, getObjectIdsByObjectTypes, moveDir } = require('./utils');
const { broadcastFromTxBuilder } = require('./sign-utils');

async function deployTokenFromInfo(client, keypair, symbol, name, decimals, walletAddress) {
    if (!name || !symbol || !decimals) throw new Error('Token name, symbol and decimals are required');

    // Define the interchain token options
    copyMovePackage('interchain_token', null, moveDir);
    const interchainTokenOptions = {
        symbol,
        name,
        decimals,
    };

    const txBuilder = new TxBuilder(client);

    // Token Capability
    const tokenCap = await txBuilder.publishInterchainToken(moveDir, interchainTokenOptions);
    txBuilder.tx.transferObjects([tokenCap], walletAddress);

    // Publish token and derive type
    const publishTxn = await broadcastFromTxBuilder(txBuilder, keypair, `Published ${symbol}`, options);
    const publishObject = findPublishedObject(publishTxn);
    const packageId = publishObject.packageId;
    const tokenType = `${packageId}::${symbol.toLowerCase()}::${symbol.toUpperCase()}`;
    const [treasuryCap, metadata] = getObjectIdsByObjectTypes(publishTxn, [`TreasuryCap<${tokenType}>`, `Metadata<${tokenType}>`]);

    return { metadata, packageId, tokenType, treasuryCap };
}

async function newCoinManagementLocked(itsConfig, tokenType, walletAddress) {
    const tx = new Transaction();
    const coinManagement = tx.moveCall({
        target: `${itsConfig.address}::interchain_token_service::coin_management::new_locked`,
        typeArguments: [tokenType],
    });
    tx.transferObjects([coinManagement], walletAddress);

    return coinManagement;
}

async function saveTokenDeployment(packageId, contracts, symbol, TokenId, TreasuryCap, Metadata) {
    contracts[symbol.toUpperCase()] = {
        address: packageId,
        typeArgument: tokenType,
        decimals,
        objects: {
            TokenId,
            TreasuryCap,
            Metadata,
        },
    };
}

module.exports = {
    deployTokenFromInfo,
    newCoinManagementLocked,
    saveTokenDeployment,
};
