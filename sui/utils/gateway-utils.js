const { TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { broadcastFromTxBuilder } = require('./sign-utils');

async function newChannel(config, gatewayAddress) {
    const txBuilder = new TxBuilder(config.client);

    const channel = await txBuilder.moveCall({
        target: `${gatewayAddress}::channel::new`,
    });

    txBuilder.tx.transferObjects([channel], config.walletAddress);

    const result = await broadcastFromTxBuilder(txBuilder, config.keypair, `Create gateway channel`, config.options, {
        showEvents: true,
    });

    const channelId = result.events[0].parsedJson.id;

    return channelId;
}

module.exports = { newChannel };
