const { Ed25519Keypair } = require('@mysten/sui/keypairs/ed25519');
const { STD_PACKAGE_ID, SUI_PACKAGE_ID, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { broadcastFromTxBuilder } = require('./sign-utils');
const { isAllowed, suiClockAddress } = require('./utils');

async function registerCustomCoinUtil(config, itsConfig, AxelarGateway, coinSymbol, coinMetadata, coinType, treasuryCap = null) {
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
    const saltAddress = createSaltAddress();
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
    if (config.options.treasuryCap) {
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

function createSaltAddress() {
    const keypair = new Ed25519Keypair();
    const address = keypair.getPublicKey().toSuiAddress();
    return address;
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

async function mockItsFunction(keypair, client, options, chain, itsConfig, fnName = '', version = '') {
    const { InterchainTokenService } = itsConfig.objects;

    // Mock Coin settings
    const coinType =
        options.env === 'mainnet'
            ? '0x4f72b86067e14066628d2ab53b31d1b96725daf44b9ae3f3686d783fdab232b3::tst::TST'
            : '0xef0980a9ecbc2dabbe865d95124929cbda72238def9e8242a702459f49818f5b::COIN::COIN';
    const coinMetadata =
        options.env === 'mainnet'
            ? '0xd8386847249c6fd543221287b39727a9869d05376dd5df1c7349bda576ec9e4b'
            : '0x46928f514ba43818062f3e05b2c42d4331a6c3e0fa88fb939f49d232b34b6091';
    const treasuryCapReclaimerType = [itsConfig.structs.TreasuryCapReclaimer, '<', coinType, '>'].join('');

    if (!itsFunctions[String(version)]) throw new Error(`Invalid version: ${String(version)}`);
    else if (itsFunctions[version].indexOf(String(fnName)) < 0) throw new Error(`Unsupported function name: ${String(fnName)}`);

    try {
        switch (fnName) {
            case 'register_coin': {
                const register_coin = (tx) => {
                    const coinManagement = tx.moveCall({
                        target: `${itsConfig.address}::coin_management::new_locked`,
                        typeArguments: [coinType],
                    });

                    const coinInfo = tx.moveCall({
                        target: `${itsConfig.address}::coin_info::from_info`,
                        arguments: [tx.pure.string(''), tx.pure.string(''), tx.pure.string('')],
                        typeArguments: [coinType],
                    });

                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::register_coin`,
                        arguments: [tx.object(InterchainTokenService), coinInfo, coinManagement],
                        typeArguments: [coinType],
                    });
                };
                return await isAllowed(client, keypair, chain, register_coin, options);
            }
            case 'register_coin_from_info': {
                const register_coin_from_info = (tx) => {
                    const coinManagement = tx.moveCall({
                        target: `${itsConfig.address}::coin_management::new_locked`,
                        typeArguments: [coinType],
                    });

                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::register_coin_from_info`,
                        arguments: [
                            tx.object(InterchainTokenService),
                            tx.pure.string(''),
                            tx.pure.string(''),
                            tx.pure.string(''),
                            coinManagement,
                        ],
                        typeArguments: [coinType],
                    });
                };
                return await isAllowed(client, keypair, chain, register_coin_from_info, options);
            }
            case 'register_coin_from_metadata': {
                const register_coin_from_metadata = (tx) => {
                    const coinManagement = tx.moveCall({
                        target: `${itsConfig.address}::coin_management::new_locked`,
                        typeArguments: [coinType],
                    });

                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::register_coin_from_metadata`,
                        arguments: [tx.object(InterchainTokenService), tx.object(coinMetadata), coinManagement],
                        typeArguments: [coinType],
                    });
                };
                return await isAllowed(client, keypair, chain, register_coin_from_metadata, options);
            }
            case 'register_custom_coin': {
                const register_custom_coin = (tx) => {
                    const coinManagement = tx.moveCall({
                        target: `${itsConfig.address}::coin_management::new_locked`,
                        typeArguments: [coinType],
                    });

                    const channel = tx.moveCall({
                        target: `${chain.contracts.AxelarGateway.address}::channel::new`,
                        arguments: [],
                    });

                    const salt = tx.moveCall({
                        target: `${chain.contracts.AxelarGateway.address}::bytes32::new`,
                        arguments: [tx.pure.address('0x0')],
                    });

                    const [_tokenId, treasuryCapReclaimerOption] = tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::register_custom_coin`,
                        arguments: [tx.object(InterchainTokenService), channel, salt, tx.object(coinMetadata), coinManagement],
                        typeArguments: [coinType],
                    });

                    tx.moveCall({
                        target: `${STD_PACKAGE_ID}::option::destroy_none`,
                        arguments: [treasuryCapReclaimerOption],
                        typeArguments: [treasuryCapReclaimerType],
                    });
                };
                return await isAllowed(client, keypair, chain, register_custom_coin, options);
            }
            case 'link_coin': {
                const link_coin = (tx) => {
                    const channel = tx.moveCall({
                        target: `${chain.contracts.AxelarGateway.address}::channel::new`,
                        arguments: [],
                    });

                    const salt = tx.moveCall({
                        target: `${chain.contracts.AxelarGateway.address}::bytes32::new`,
                        arguments: [tx.pure.address('0x0')],
                    });

                    const tokenManagerType = tx.moveCall({
                        target: `${itsConfig.address}::token_manager_type::lock_unlock`,
                    });

                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::link_coin`,
                        arguments: [
                            tx.object(InterchainTokenService),
                            channel,
                            salt,
                            tx.pure.string(''),
                            tx.pure.vector('u8', []),
                            tokenManagerType,
                            tx.pure.vector('u8', []),
                        ],
                    });
                };
                return await isAllowed(client, keypair, chain, link_coin, options);
            }
            case 'register_coin_metadata': {
                const register_coin_metadata = (tx) => {
                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::register_coin_metadata`,
                        arguments: [tx.object(InterchainTokenService), tx.object(coinMetadata)],
                        typeArguments: [coinType],
                    });
                };
                return await isAllowed(client, keypair, chain, register_coin_metadata, options);
            }
            case 'deploy_remote_interchain_token': {
                const deploy_remote_interchain_token = (tx) => {
                    const coinManagement = tx.moveCall({
                        target: `${itsConfig.address}::coin_management::new_locked`,
                        typeArguments: [coinType],
                    });

                    let tokenId;
                    if (version == 0) {
                        const coinInfo = tx.moveCall({
                            target: `${itsConfig.address}::coin_info::from_info`,
                            arguments: [tx.pure.string(''), tx.pure.string(''), tx.pure.string('')],
                            typeArguments: [coinType],
                        });

                        tokenId = tx.moveCall({
                            target: `${itsConfig.address}::interchain_token_service::register_coin`,
                            arguments: [tx.object(InterchainTokenService), coinInfo, coinManagement],
                            typeArguments: [coinType],
                        });
                    } else {
                        tokenId = tx.moveCall({
                            target: `${itsConfig.address}::interchain_token_service::register_coin_from_info`,
                            arguments: [
                                tx.object(InterchainTokenService),
                                tx.pure.string(''),
                                tx.pure.string(''),
                                tx.pure.string(''),
                                coinManagement,
                            ],
                            typeArguments: [coinType],
                        });
                    }

                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::deploy_remote_interchain_token`,
                        arguments: [tx.object(InterchainTokenService), tokenId, tx.pure.string('')],
                        typeArguments: [coinType],
                    });
                };
                return await isAllowed(client, keypair, chain, deploy_remote_interchain_token, options);
            }
            case 'give_unlinked_coin': {
                const give_unlinked_coin = (tx) => {
                    const tokenIdObject = tx.moveCall({
                        target: `${itsConfig.address}::token_id::from_address`,
                        arguments: [tx.pure.address('0x0')],
                    });

                    const treasuryCapOption = tx.moveCall({
                        target: `${STD_PACKAGE_ID}::option::none`,
                        arguments: [],
                        typeArguments: [`${SUI_PACKAGE_ID}::coin::TreasuryCap<${coinType}>`],
                    });

                    const treasuryCapReclaimerOption = tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::give_unlinked_coin`,
                        arguments: [tx.object(InterchainTokenService), tokenIdObject, tx.object(coinMetadata), treasuryCapOption],
                        typeArguments: [coinType],
                    });

                    tx.moveCall({
                        target: `${STD_PACKAGE_ID}::option::destroy_none`,
                        arguments: [treasuryCapReclaimerOption],
                        typeArguments: [treasuryCapReclaimerType],
                    });
                };
                return await isAllowed(client, keypair, chain, give_unlinked_coin, options);
            }
            case 'add_trusted_chains': {
                const add_trusted_chains = (tx) => {
                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::add_trusted_chains`,
                        arguments: [tx.object(InterchainTokenService), tx.object(itsConfig.objects.OwnerCap), tx.pure.vector('string', [])],
                    });
                };
                return await isAllowed(client, keypair, chain, add_trusted_chains, options);
            }
            case 'remove_trusted_chains': {
                const remove_trusted_chains = (tx) => {
                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::remove_trusted_chains`,
                        arguments: [tx.object(InterchainTokenService), tx.object(itsConfig.objects.OwnerCap), tx.pure.vector('string', [])],
                    });
                };
                return await isAllowed(client, keypair, chain, remove_trusted_chains, options);
            }
            case 'set_flow_limit': {
                const set_flow_limit = (tx) => {
                    const coinManagement = tx.moveCall({
                        target: `${itsConfig.address}::coin_management::new_locked`,
                        typeArguments: [coinType],
                    });

                    let tokenId;
                    if (version == 0) {
                        const coinInfo = tx.moveCall({
                            target: `${itsConfig.address}::coin_info::from_info`,
                            arguments: [tx.pure.string(''), tx.pure.string(''), tx.pure.string('')],
                            typeArguments: [coinType],
                        });

                        tokenId = tx.moveCall({
                            target: `${itsConfig.address}::interchain_token_service::register_coin`,
                            arguments: [tx.object(InterchainTokenService), coinInfo, coinManagement],
                            typeArguments: [coinType],
                        });
                    } else {
                        tokenId = tx.moveCall({
                            target: `${itsConfig.address}::interchain_token_service::register_coin_from_info`,
                            arguments: [
                                tx.object(InterchainTokenService),
                                tx.pure.string(''),
                                tx.pure.string(''),
                                tx.pure.string(''),
                                coinManagement,
                            ],
                            typeArguments: [coinType],
                        });
                    }

                    const limitOption = tx.moveCall({
                        target: `${STD_PACKAGE_ID}::option::none`,
                        arguments: [],
                        typeArguments: ['u64'],
                    });

                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::set_flow_limit`,
                        arguments: [tx.object(InterchainTokenService), tx.object(itsConfig.objects.OperatorCap), tokenId, limitOption],
                        typeArguments: [coinType],
                    });
                };
                return await isAllowed(client, keypair, chain, set_flow_limit, options);
            }
            case 'set_flow_limit_as_token_operator': {
                const set_flow_limit_as_token_operator = (tx) => {
                    const coinManagement = tx.moveCall({
                        target: `${itsConfig.address}::coin_management::new_locked`,
                        typeArguments: [coinType],
                    });

                    const channel = tx.moveCall({
                        target: `${chain.contracts.AxelarGateway.address}::channel::new`,
                        arguments: [],
                    });

                    let tokenId;
                    if (version == 0) {
                        const coinInfo = tx.moveCall({
                            target: `${itsConfig.address}::coin_info::from_info`,
                            arguments: [tx.pure.string(''), tx.pure.string(''), tx.pure.string('')],
                            typeArguments: [coinType],
                        });

                        tokenId = tx.moveCall({
                            target: `${itsConfig.address}::interchain_token_service::register_coin`,
                            arguments: [tx.object(InterchainTokenService), coinInfo, coinManagement],
                            typeArguments: [coinType],
                        });
                    } else {
                        tokenId = tx.moveCall({
                            target: `${itsConfig.address}::interchain_token_service::register_coin_from_info`,
                            arguments: [
                                tx.object(InterchainTokenService),
                                tx.pure.string(''),
                                tx.pure.string(''),
                                tx.pure.string(''),
                                coinManagement,
                            ],
                            typeArguments: [coinType],
                        });
                    }

                    const limitOption = tx.moveCall({
                        target: `${STD_PACKAGE_ID}::option::none`,
                        arguments: [],
                        typeArguments: ['u64'],
                    });

                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::set_flow_limit_as_token_operator`,
                        arguments: [tx.object(InterchainTokenService), channel, tokenId, limitOption],
                        typeArguments: [coinType],
                    });
                };
                return await isAllowed(client, keypair, chain, set_flow_limit_as_token_operator, options);
            }
            case 'transfer_operatorship': {
                const transfer_operatorship = (tx) => {
                    const coinManagement = tx.moveCall({
                        target: `${itsConfig.address}::coin_management::new_locked`,
                        typeArguments: [coinType],
                    });

                    const channel = tx.moveCall({
                        target: `${chain.contracts.AxelarGateway.address}::channel::new`,
                        arguments: [],
                    });

                    const channelAddress = tx.moveCall({
                        target: `${chain.contracts.AxelarGateway.address}::channel::to_address`,
                        arguments: [channel],
                    });

                    tx.moveCall({
                        target: `${itsConfig.address}::coin_management::add_operator`,
                        arguments: [coinManagement, channelAddress],
                        typeArguments: [coinType],
                    });

                    const tokenId = tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::register_coin_from_metadata`,
                        arguments: [tx.object(InterchainTokenService), tx.object(coinMetadata), coinManagement],
                        typeArguments: [coinType],
                    });

                    const operatorAddressOption = tx.moveCall({
                        target: `${STD_PACKAGE_ID}::option::some`,
                        arguments: [tx.pure.address('0x0')],
                        typeArguments: ['address'],
                    });

                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::transfer_operatorship`,
                        arguments: [tx.object(InterchainTokenService), channel, tokenId, operatorAddressOption],
                        typeArguments: [coinType],
                    });
                };
                return await isAllowed(client, keypair, chain, transfer_operatorship, options);
            }
            case 'allow_function': {
                const allow_function = (tx) => {
                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::allow_function`,
                        arguments: [
                            tx.object(InterchainTokenService),
                            tx.object(itsConfig.objects.OwnerCap),
                            tx.pure.u64(parseInt(version)),
                            tx.pure.string(''),
                        ],
                    });
                };
                return await isAllowed(client, keypair, chain, allow_function, options);
            }
            case 'disallow_function': {
                const disallow_function = (tx) => {
                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::disallow_function`,
                        arguments: [
                            tx.object(InterchainTokenService),
                            tx.object(itsConfig.objects.OwnerCap),
                            tx.pure.u64(parseInt(version)),
                            tx.pure.string(''),
                        ],
                    });
                };
                return await isAllowed(client, keypair, chain, disallow_function, options);
            }
            case 'migrate_coin_metadata': {
                const migrate_coin_metadata = (tx) => {
                    tx.moveCall({
                        target: `${itsConfig.address}::interchain_token_service::migrate_coin_metadata`,
                        arguments: [tx.object(InterchainTokenService), tx.object(itsConfig.objects.OperatorCap), tx.pure.address('0x0')],
                        typeArguments: [coinType],
                    });
                };
                return await isAllowed(client, keypair, chain, migrate_coin_metadata, options);
            }
            // Validation is skipped for the following fns that would require gas
            case 'send_interchain_transfer':
            case 'receive_interchain_transfer':
            case 'receive_interchain_transfer_with_data': {
                return { skipped: true, reason: 'requires an account with owned Coin<T>' };
            }
            case 'receive_deploy_interchain_token':
            case 'receive_link_coin': {
                return { skipped: true, reason: 'requires an axelar_gateway::channel::ApprovedMessage' };
            }
            case 'register_transaction': {
                return { skipped: true, reason: 'requires a valid relayer_discovery::transaction::Transaction' };
            }
            case 'remove_unlinked_coin':
            case 'mint_as_distributor':
            case 'mint_to_as_distributor':
            case 'burn_as_distributor':
            case 'give_unregistered_coin':
            case 'transfer_distributorship':
            case 'remove_treasury_cap':
            case 'restore_treasury_cap': {
                return { skipped: true, reason: 'requires a valid TreasuryCap' };
            }
            default: {
                return false;
            }
        }
    } catch {
        return false;
    }
}

module.exports = { createSaltAddress, registerCustomCoinUtil, itsFunctions, mockItsFunction };
