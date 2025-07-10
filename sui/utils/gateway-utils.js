const { TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { broadcastFromTxBuilder } = require('./sign-utils');

async function newChannel(config, gatewayAddress) {
    const txBuilder = new TxBuilder(config.client);

    const channel = await txBuilder.moveCall({
        target: `${gatewayAddress}::channel::new`,
    });

    const deployerChannelAddress = await txBuilder.moveCall({
        target: `${gatewayAddress}::channel::to_address`,
        arguments: [channel],
    });

    txBuilder.tx.transferObjects([channel], config.walletAddress);

    const result = await broadcastFromTxBuilder(txBuilder, config.keypair, `Create gateway channel`, config.options, {
            showEvents: true,
        }
    );
    
    const channelId = result.events[0].parsedJson.id;

    return [channelId, deployerChannelAddress];
}

module.exports = { newChannel };