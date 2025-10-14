const { Ed25519Keypair } = require('@mysten/sui/keypairs/ed25519');
const { STD_PACKAGE_ID, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { Transaction } = require('@mysten/sui/transactions');
const { bcs } = require('@mysten/sui/bcs');
const { broadcastFromTxBuilder } = require('./sign-utils');

async function registerCustomCoinUtil(
    config,
    itsConfig,
    AxelarGateway,
    coinSymbol,
    coinMetadata,
    coinType,
    treasuryCap = null,
    fixedSalt = null,
) {
    const { InterchainTokenService } = itsConfig.objects;
    const txBuilder = new TxBuilder(config.client);

    // New CoinManagement<T>
    const coinManagement = !treasuryCap
        ? await txBuilder.moveCall({
              target: `${itsConfig.address}::coin_management::new_locked`,
              typeArguments: [coinType],
          })
        : await txBuilder.moveCall({
              target: `${itsConfig.address}::coin_management::new_with_cap`,
              arguments: [treasuryCap],
              typeArguments: [coinType],
          });

    // Channel
    const channel = config.options.channel
        ? config.options.channel
        : await txBuilder.moveCall({
              target: `${AxelarGateway.address}::channel::new`,
          });

    // Salt
    const saltAddress = fixedSalt ? fixedSalt : createSaltAddress();
    const salt = await txBuilder.moveCall({
        target: `${AxelarGateway.address}::bytes32::new`,
        arguments: [saltAddress],
    });

    // Register deployed token (from info)
    const [_tokenId, treasuryCapReclaimerOption] = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::register_custom_coin`,
        arguments: [InterchainTokenService, channel, salt, coinMetadata, coinManagement],
        typeArguments: [coinType],
    });

    // TreasuryCapReclaimer<T>
    const treasuryCapReclaimerType = [itsConfig.structs.TreasuryCapReclaimer, '<', coinType, '>'].join('');
    if (treasuryCap) {
        const treasuryCapReclaimer = await txBuilder.moveCall({
            target: `${STD_PACKAGE_ID}::option::extract`,
            arguments: [treasuryCapReclaimerOption],
            typeArguments: [treasuryCapReclaimerType],
        });

        txBuilder.tx.transferObjects([treasuryCapReclaimer], config.walletAddress);
    }
    await txBuilder.moveCall({
        target: `${STD_PACKAGE_ID}::option::destroy_none`,
        arguments: [treasuryCapReclaimerOption],
        typeArguments: [treasuryCapReclaimerType],
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

    return [tokenId, channelId, saltAddress, result];
}

function createSaltAddress(keypair = null) {
    if (!keypair) {
        keypair = new Ed25519Keypair();
    }
    const address = keypair.getPublicKey().toSuiAddress();
    return address;
}

async function tokenIdToCoinType(client, config, tokenId = '') {
    try {
        const coinTypeResult = await client.devInspectTransactionBlock({
            transactionBlock: (() => {
                const tx = new Transaction();
                tx.moveCall({
                    target: `${config.itsConfig.address}::interchain_token_service::registered_coin_type`,
                    arguments: [
                        tx.object(config.itsConfig.objects.InterchainTokenService),
                        tx.pure.address(tokenId)
                    ],
                });
                return tx;
            })(),
            sender: config.walletAddress,
        });

        const coinType = extractCoinTypeFromDevInspect(coinTypeResult);
    } catch {
        throw new Error(`Failed parsing coin type for token id ${tokenId}`);
    }
}

function extractCoinTypeFromDevInspect(result) {
    if (result.results?.[0]?.returnValues?.[0]) {
        const [bytes] = result.results[0].returnValues[0];
        const coinType = bcs.String.parse(new Uint8Array(bytes));
        return coinType;
    }
    throw new Error(`Failed to get coin type from dev inspect for result ${result}`);
}

const itsFunctions = {
    0: [
        'register_coin',
        'deploy_remote_interchain_token',
        'send_interchain_transfer',
        'receive_interchain_transfer',
        'receive_interchain_transfer_with_data',
        'receive_deploy_interchain_token',
        'give_unregistered_coin',
        'mint_as_distributor',
        'mint_to_as_distributor',
        'burn_as_distributor',
        'add_trusted_chains',
        'remove_trusted_chains',
        'register_transaction',
        'set_flow_limit',
        'set_flow_limit_as_token_operator',
        'transfer_distributorship',
        'transfer_operatorship',
        'allow_function',
        'disallow_function',
    ],
    1: [
        'register_coin_from_info',
        'register_coin_from_metadata',
        'register_custom_coin',
        'link_coin',
        'register_coin_metadata',
        'deploy_remote_interchain_token',
        'send_interchain_transfer',
        'receive_interchain_transfer',
        'receive_interchain_transfer_with_data',
        'receive_deploy_interchain_token',
        'receive_link_coin',
        'give_unregistered_coin',
        'give_unlinked_coin',
        'remove_unlinked_coin',
        'mint_as_distributor',
        'mint_to_as_distributor',
        'burn_as_distributor',
        'add_trusted_chains',
        'remove_trusted_chains',
        'register_transaction',
        'set_flow_limit',
        'set_flow_limit_as_token_operator',
        'transfer_distributorship',
        'transfer_operatorship',
        'remove_treasury_cap',
        'restore_treasury_cap',
        'allow_function',
        'disallow_function',
        'migrate_coin_metadata',
    ],
};

module.exports = { 
    createSaltAddress,
    extractCoinTypeFromDevInspect,
    registerCustomCoinUtil,
    tokenIdToCoinType,
    itsFunctions 
};
