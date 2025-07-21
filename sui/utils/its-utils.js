const { Ed25519Keypair } = require('@mysten/sui/keypairs/ed25519');
const { STD_PACKAGE_ID, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { broadcastFromTxBuilder } = require('./sign-utils');

async function registerCustomCoinUtil(config, itsConfig, AxelarGateway, coinSymbol, coinMetadata, coinType) {
    const { InterchainTokenService } = itsConfig.objects;
    const txBuilder = new TxBuilder(config.client);

    // New CoinManagement<T>
    const coinManagement = await txBuilder.moveCall({
        target: `${itsConfig.address}::coin_management::new_locked`,
        typeArguments: [coinType],
    });

    // Channel
    const channel = config.options.channel
        ? config.options.channel
        : await txBuilder.moveCall({
              target: `${AxelarGateway.address}::channel::new`,
          });

    // Salt
    const salt = await txBuilder.moveCall({
        target: `${AxelarGateway.address}::bytes32::new`,
        arguments: [createSaltAddress()],
    });

    // Register deployed token (from info)
    const [_tokenId, treasuryCapReclaimer] = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::register_custom_coin`,
        arguments: [InterchainTokenService, channel, salt, coinMetadata, coinManagement],
        typeArguments: [coinType],
    });

    await txBuilder.moveCall({
        target: `${STD_PACKAGE_ID}::option::destroy_none`,
        arguments: [treasuryCapReclaimer],
        typeArguments: [[itsConfig.structs.TreasuryCapReclaimer, '<', coinType, '>'].join('')],
    });

    if (!config.options.channel) txBuilder.tx.transferObjects([channel], config.walletAddress);

    const result = await broadcastFromTxBuilder(
        txBuilder,
        config.keypair,
        `Register Custom Coin (${coinSymbol}) in InterchainTokenService`,
        config.options,
        {
            showEvents: true,
        },
    );

    let tokenEvent = result.events.filter((evt) => {
        return evt.parsedJson.token_id ? true : false;
    })[0];

    let channelEvent = result.events.filter((evt) => {
        return evt.transactionModule == 'channel' ? true : false;
    })[0];

    if (!tokenEvent) tokenEvent = { parsedJson: {} };
    if (!channelEvent) channelEvent = { parsedJson: {} };

    const tokenId = tokenEvent.parsedJson.hasOwnProperty('token_id') ? tokenEvent.parsedJson.token_id.id : null;
    const channelId = channelEvent.parsedJson.hasOwnProperty('id') ? channelEvent.parsedJson.id : null;

    return [tokenId, channelId];
}

function createSaltAddress() {
    const keypair = new Ed25519Keypair();
    const address = keypair.getPublicKey().toSuiAddress();
    return address;
}

module.exports = { createSaltAddress, registerCustomCoinUtil };
