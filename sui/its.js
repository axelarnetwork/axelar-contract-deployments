const { Option, Command } = require('commander');
const { STD_PACKAGE_ID, SUI_PACKAGE_ID, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const {
    loadConfig,
    printInfo,
    saveConfig,
    getChainConfig,
    parseTrustedChains,
    validateParameters,
    validateDestinationChain,
    estimateITSFee,
    encodeITSDestinationToken,
    encodeITSDestination,
} = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    broadcastFromTxBuilder,
    createLockedCoinManagement,
    deployTokenFromInfo,
    getAllowedFunctions,
    getObjectIdsByObjectTypes,
    getWallet,
    getFormattedAmount,
    itsFunctions,
    printWalletInfo,
    registerCustomCoinUtil,
    saveGeneratedTx,
    saveTokenDeployment,
    suiClockAddress,
    suiCoinId,
    getUnitAmount,
    getBagContents,
    tokenIdToCoinType,
} = require('./utils');
const { bcs } = require('@mysten/sui/bcs');
const chalk = require('chalk');
const {
    utils: { arrayify, parseUnits },
} = require('hardhat').ethers;
const { checkIfCoinExists, senderHasSufficientBalance } = require('./utils/token-utils');

async function setFlowLimits(keypair, client, config, contracts, args, options) {
    let [tokenIds, flowLimits] = args;

    const { InterchainTokenService: itsConfig } = contracts;

    const { OperatorCap, InterchainTokenService } = itsConfig.objects;

    const txBuilder = new TxBuilder(client);

    tokenIds = tokenIds.split(',');
    flowLimits = flowLimits.split(',');

    if (tokenIds.length !== flowLimits.length) {
        throw new Error('<token-ids> and <flow-limits> have to have the same length.');
    }

    for (let i = 0; i < tokenIds.length; i++) {
        const coinTypeTxBuilder = new TxBuilder(client);
        let tokenId = await coinTypeTxBuilder.moveCall({
            target: `${itsConfig.address}::token_id::from_address`,
            arguments: [tokenIds[i]],
        });

        await coinTypeTxBuilder.moveCall({
            target: `${itsConfig.address}::interchain_token_service::registered_coin_type`,
            arguments: [InterchainTokenService, tokenId],
        });

        const resp = await coinTypeTxBuilder.devInspect(keypair.toSuiAddress());
        const coinType = bcs.String.parse(new Uint8Array(resp.results[1].returnValues[0][0]));

        tokenId = await txBuilder.moveCall({
            target: `${itsConfig.address}::token_id::from_address`,
            arguments: [tokenIds[i]],
        });

        let flowLimit;

        if (flowLimits[i] === 'none') {
            flowLimit = await txBuilder.moveCall({
                target: `${STD_PACKAGE_ID}::option::none`,
                arguments: [],
                typeArguments: ['u64'],
            });
        } else {
            flowLimit = await txBuilder.moveCall({
                target: `${STD_PACKAGE_ID}::option::some`,
                arguments: [txBuilder.tx.pure.u64(Number(flowLimits[i]))],
                typeArguments: ['u64'],
            });
        }

        await txBuilder.moveCall({
            target: `${itsConfig.address}::interchain_token_service::set_flow_limit`,
            arguments: [InterchainTokenService, OperatorCap, tokenId, flowLimit],
            typeArguments: [coinType],
        });
    }

    if (options.offline) {
        const tx = txBuilder.tx;
        const sender = options.sender || keypair.toSuiAddress();
        tx.setSender(sender);
        await saveGeneratedTx(tx, `Set flow limits for ${tokenIds} to ${flowLimits}`, client, options);
    } else {
        await broadcastFromTxBuilder(txBuilder, keypair, 'Set flow limits', options);
    }
}

async function addTrustedChains(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;

    const { OwnerCap, InterchainTokenService } = itsConfig.objects;

    let trustedChains = parseTrustedChains(config.chains, args);

    if (!options.offline) {
        const alreadyTrustedChains = await listTrustedChains(keypair, client, config, contracts, args, options);

        trustedChains = trustedChains.filter((chain) => !alreadyTrustedChains.includes(chain));

        if (trustedChains.length === 0) {
            printInfo('All specified chains are already trusted. No action needed.');
            return;
        }
    }

    printInfo('Chains to add as trusted', trustedChains);

    const txBuilder = new TxBuilder(client);

    await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::add_trusted_chains`,
        arguments: [InterchainTokenService, OwnerCap, trustedChains],
    });

    if (options.offline) {
        const tx = txBuilder.tx;
        const sender = options.sender || keypair.toSuiAddress();
        tx.setSender(sender);
        await saveGeneratedTx(tx, `Added trusted chains ${args}`, client, options);
    } else {
        await broadcastFromTxBuilder(txBuilder, keypair, 'Add Trusted Chains', options);
    }
}
async function removeTrustedChains(keypair, client, config, contracts, args, options) {
    const trustedChains = args;

    if (trustedChains.length === 0) {
        throw new Error('No chains names provided');
    }

    const txBuilder = new TxBuilder(client);

    await txBuilder.moveCall({
        target: `${contracts.InterchainTokenService.address}::interchain_token_service::remove_trusted_chains`,
        arguments: [
            contracts.InterchainTokenService.objects.InterchainTokenService,
            contracts.InterchainTokenService.objects.OwnerCap,
            trustedChains,
        ],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, 'Remove Trusted Chains', options);
}

async function registerCoinFromInfo(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { InterchainTokenService } = itsConfig.objects;
    const [symbol, name, decimals] = args;

    const walletAddress = keypair.toSuiAddress();
    const deployConfig = { client, keypair, options, walletAddress };

    // Deploy token on Sui
    const [metadata, packageId, tokenType, treasuryCap] = await deployTokenFromInfo(deployConfig, symbol, name, decimals);

    // New CoinManagement<T>
    const [txBuilder, coinManagement] = await createLockedCoinManagement(deployConfig, itsConfig, tokenType);

    // Register deployed token (from info)
    await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::register_coin_from_info`,
        arguments: [InterchainTokenService, name, symbol, decimals, coinManagement],
        typeArguments: [tokenType],
    });

    const result = await broadcastFromTxBuilder(
        txBuilder,
        keypair,
        `Register coin (${symbol}) from info in InterchainTokenService`,
        options,
        {
            showEvents: true,
        },
    );

    const tokenId = result.events[0].parsedJson.token_id.id;

    // Save the deployed token
    saveTokenDeployment(packageId, tokenType, contracts, symbol, decimals, tokenId, treasuryCap, metadata);
}

async function registerCoinFromMetadata(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { InterchainTokenService } = itsConfig.objects;
    const [symbol, name, decimals] = args;

    const walletAddress = keypair.toSuiAddress();
    const deployConfig = { client, keypair, options, walletAddress };

    // Deploy token on Sui
    const [metadata, packageId, tokenType, treasuryCap] = await deployTokenFromInfo(deployConfig, symbol, name, decimals);

    // New CoinManagement<T>
    const [txBuilder, coinManagement] = await createLockedCoinManagement(deployConfig, itsConfig, tokenType);

    // Register deployed token (from metadata)
    await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::register_coin_from_metadata`,
        arguments: [InterchainTokenService, metadata, coinManagement],
        typeArguments: [tokenType],
    });

    const result = await broadcastFromTxBuilder(
        txBuilder,
        keypair,
        `Register coin (${symbol}) from Coin Metadata in InterchainTokenService`,
        options,
        {
            showEvents: true,
        },
    );
    const tokenId = result.events[0].parsedJson.token_id.id;

    // Save the deployed token
    saveTokenDeployment(packageId, tokenType, contracts, symbol, decimals, tokenId, treasuryCap, metadata);
}

async function registerCustomCoin(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig, AxelarGateway } = contracts;
    const walletAddress = keypair.toSuiAddress();
    const deployConfig = { client, keypair, options, walletAddress };
    const [symbol, name, decimals] = args;

    if (options.salt) {
        validateParameters({
            isHexString: { salt: options.salt },
        });
    }

    const coin = options.published ? contracts[symbol.toUpperCase()] : null;
    if (!coin && options.published) {
        throw new Error(`Cannot find coin with symbol ${symbol} in config`);
    }

    // Deploy token on Sui
    const [metadata, packageId, tokenType, treasuryCap] = options.published
        ? [coin.objects.Metadata, coin.address, coin.typeArgument, coin.objects.TreasuryCap]
        : await deployTokenFromInfo(deployConfig, symbol, name, decimals);

    // Mint pre-registration coins
    const amount = Number.isFinite(Number(options.mintAmount)) ? parseInt(options.mintAmount) : 0;
    if (amount) {
        const unitAmount = getUnitAmount(options.mintAmount, decimals);

        const mintTxBuilder = new TxBuilder(client);

        const coin = await mintTxBuilder.moveCall({
            target: `${SUI_PACKAGE_ID}::coin::mint`,
            arguments: [treasuryCap, unitAmount],
            typeArguments: [tokenType],
        });

        mintTxBuilder.tx.transferObjects([coin], walletAddress);

        await broadcastFromTxBuilder(mintTxBuilder, keypair, `Minted ${amount} ${symbol}`, options);
    }

    // Register deployed token (custom)
    const [tokenId, _channelId, saltAddress, result] = await registerCustomCoinUtil(
        deployConfig,
        itsConfig,
        AxelarGateway,
        symbol,
        metadata,
        tokenType,
        options.treasuryCap ? treasuryCap : null,
        options.salt ? options.salt : null,
    );
    if (!tokenId) {
        throw new Error(`error resolving token id from registration tx, got ${tokenId}`);
    }

    // Save the deployed token
    saveTokenDeployment(packageId, tokenType, contracts, symbol, decimals, tokenId, treasuryCap, metadata, [], saltAddress);

    // Save TreasuryCapReclaimer to coin config (if exists)
    if (options.treasuryCap && contracts[symbol.toUpperCase()]) {
        const [treasuryCapReclaimerId] = getObjectIdsByObjectTypes(result, [`TreasuryCapReclaimer<${tokenType}>`]);
        contracts[symbol.toUpperCase()].objects.TreasuryCapReclaimer = treasuryCapReclaimerId;
    }
}
async function listTrustedChains(_keypair, client, _config, contracts, _args, _options) {
    const { InterchainTokenService: itsConfig } = contracts;

    // Use the v0 value object to read on-chain state
    const { InterchainTokenServicev0 } = itsConfig.objects;

    const itsObject = await client.getObject({
        id: InterchainTokenServicev0,
        options: { showContent: true },
    });

    // trusted_chains: TrustedChains { trusted_chains: Bag { id } }
    const bagId = itsObject?.data?.content?.fields?.value?.fields?.trusted_chains?.fields?.trusted_chains?.fields?.id?.id;

    if (!bagId) {
        throw new Error(`Unable to locate trusted_chains bag for ITS object ${InterchainTokenServicev0}`);
    }

    const loadChainName = (entry) => {
        const name =
            entry?.name && typeof entry.name === 'object' && 'value' in entry.name
                ? entry.name.value
                : typeof entry.name === 'string'
                  ? entry.name
                  : JSON.stringify(entry.name);
        return name;
    };

    const chains = await getBagContents(client, bagId, loadChainName);

    printInfo('Trusted chains', chains);
    return chains;
}

async function migrateAllCoinMetadata(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { OperatorCap, InterchainTokenService } = itsConfig.objects;

    // Show or disable logging output (0 = logging disabled)
    let logSize = options.logging ? parseInt(options.logging) : 0;
    if (isNaN(logSize)) {
        logSize = 0;
    }

    // Batch or process txs 1-by-1 (0 = batching disabled)
    let batchSize = options.batch ? parseInt(options.batch) : 0;
    if (isNaN(batchSize)) {
        batchSize = 0;
    }

    // Migrate all the coins. This might take a while.
    const legacyCoins = contracts.InterchainTokenService.legacyCoins ? contracts.InterchainTokenService.legacyCoins : [];

    if (!legacyCoins.length) {
        printInfo('Warning: no migratable tokens were found in chain config for env', options.env, chalk.yellow);
    }

    let migratedCoins = [],
        failedMigrations = [],
        currentBatch = [],
        processedBatches = 0,
        txBuilder = new TxBuilder(client);
    for (let i = 0; i < legacyCoins.length; i++) {
        const coin = legacyCoins[i];

        if (batchSize) {
            currentBatch.push(coin);
        }

        try {
            const splitCoinType = coin.TokenType.split('<');
            const typeArg = splitCoinType[splitCoinType.length - 1].replace('>', '');
            await txBuilder.moveCall({
                target: `${itsConfig.address}::interchain_token_service::migrate_coin_metadata`,
                arguments: [InterchainTokenService, OperatorCap, coin.TokenId],
                typeArguments: [typeArg],
            });
            // Process tx as batch or indidivual migration (depending on options.batch)
            if (!batchSize || i == legacyCoins.length - 1 || (i + 1) % batchSize === 0) {
                // Broadcast batch / individual tx, and reset builder
                const txType = !batchSize ? coin.symbol : 'batched';
                await broadcastFromTxBuilder(txBuilder, keypair, `Migrate Coin Metadata (${txType})`, options);
                txBuilder = new TxBuilder(client);
                if (!batchSize) {
                    migratedCoins.push(coin);
                } else {
                    migratedCoins = [...migratedCoins, ...currentBatch];
                    ++processedBatches;
                    currentBatch = [];
                }
            }
        } catch (e) {
            txBuilder = new TxBuilder(client);
            if (!batchSize) {
                printInfo(`Migrate metadata failed for coin ${coin.symbol}`, e.message, chalk.red);
                failedMigrations.push(coin);
            } else {
                printInfo(`Migrate metadata failed for batch ${processedBatches}`, e.message, chalk.red);
                failedMigrations = [...failedMigrations, ...currentBatch];
                ++processedBatches;
                currentBatch = [];
            }
        }

        // Intermediate status debugging report (e.g. if options.logging enabled)
        if (logSize > 0 && (i + 1) % logSize === 0 && !batchSize) {
            printInfo(`Migrated metadata for ${migratedCoins.length} tokens. Last migrated token`, coin.symbol);
        } else if (logSize > 0 && (i + 1) % logSize === 0 && batchSize) {
            printInfo(`Migrated metadata for ${migratedCoins.length} tokens. Processed batches`, processedBatches);
        }
    }

    // Final status report
    if (migratedCoins.length) {
        printInfo('Total coins migrated', migratedCoins.length);
    } else {
        printInfo('No coins were migrated');
    }

    // Clean up chain config
    if (failedMigrations.length) {
        contracts.InterchainTokenService.legacyCoins = failedMigrations;
        printInfo('Number of failed migrations', failedMigrations.length, chalk.yellow);
    } else {
        delete contracts.InterchainTokenService.legacyCoins;
    }
}

async function migrateCoinMetadata(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { OperatorCap, InterchainTokenService } = itsConfig.objects;
    const txBuilder = new TxBuilder(client);

    const symbol = args;
    validateParameters({
        isNonEmptyString: { symbol },
        isNonArrayObject: { tokenEntry: contracts[symbol.toUpperCase()] },
    });

    const tokenId = contracts[symbol.toUpperCase()].objects.TokenId;
    const tokenType = contracts[symbol.toUpperCase()].typeArgument;

    await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::migrate_coin_metadata`,
        arguments: [InterchainTokenService, OperatorCap, tokenId],
        typeArguments: [tokenType],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, 'Migrate Coin Metadata', options);
}

async function giveUnlinkedCoin(keypair, client, _, contracts, args, options) {
    const { InterchainTokenService: itsConfig, AxelarGateway } = contracts;
    const { InterchainTokenService } = itsConfig.objects;
    const walletAddress = keypair.toSuiAddress();
    const [symbol, tokenId] = args;
    const txBuilder = new TxBuilder(client);

    validateParameters({
        isHexString: { tokenId },
    });

    const coin = contracts[symbol.toUpperCase()];
    if (!coin) {
        throw new Error(`Cannot find coin with symbol ${symbol} in config`);
    }

    const decimals = coin.decimals;
    const metadata = coin.objects.Metadata;
    const packageId = coin.address;
    const tokenType = coin.typeArgument;
    const treasuryCap = coin.objects.TreasuryCap;

    // TokenId
    const tokenIdObject = await txBuilder.moveCall({
        target: `${itsConfig.address}::token_id::from_address`,
        arguments: [tokenId],
    });

    // Option<TreasuryCap<T>>
    const target = options.treasuryCapReclaimer ? `${STD_PACKAGE_ID}::option::some` : `${STD_PACKAGE_ID}::option::none`;
    const callArguments = options.treasuryCapReclaimer ? [treasuryCap] : [];
    const typeArguments = [`${SUI_PACKAGE_ID}::coin::TreasuryCap<${tokenType}>`];
    const treasuryCapOption = await txBuilder.moveCall({ target, arguments: callArguments, typeArguments });

    // give_unlinked_coin<T>
    const [treasuryCapReclaimerOption, channelOption] = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::give_unlinked_coin`,
        arguments: [InterchainTokenService, tokenIdObject, metadata, treasuryCapOption],
        typeArguments: [tokenType],
    });

    // TreasuryCapReclaimer<T>
    const treasuryCapReclaimerType = [itsConfig.structs.TreasuryCapReclaimer, '<', tokenType, '>'].join('');
    const channelType = AxelarGateway.structs.Channel;
    if (options.treasuryCapReclaimer) {
        const treasuryCapReclaimer = await txBuilder.moveCall({
            target: `${STD_PACKAGE_ID}::option::extract`,
            arguments: [treasuryCapReclaimerOption],
            typeArguments: [treasuryCapReclaimerType],
        });

        const channel = await txBuilder.moveCall({
            target: `${STD_PACKAGE_ID}::option::extract`,
            arguments: [channelOption],
            typeArguments: [channelType],
        });

        txBuilder.tx.transferObjects([treasuryCapReclaimer, channel], walletAddress);
    }

    await txBuilder.moveCall({
        target: `${STD_PACKAGE_ID}::option::destroy_none`,
        arguments: [treasuryCapReclaimerOption],
        typeArguments: [treasuryCapReclaimerType],
    });

    await txBuilder.moveCall({
        target: `${STD_PACKAGE_ID}::option::destroy_none`,
        arguments: [channelOption],
        typeArguments: [channelType],
    });

    const result = await broadcastFromTxBuilder(txBuilder, keypair, `Give Unlinked Coin (${symbol})`, options);

    // Save the deployed token
    saveTokenDeployment(packageId, tokenType, contracts, symbol, decimals, tokenId, treasuryCap, metadata, [], '');

    // Save TreasuryCapReclaimer to coin config (if exists)
    if (options.treasuryCapReclaimer && contracts[symbol.toUpperCase()]) {
        const [treasuryCapReclaimerId] = getObjectIdsByObjectTypes(result, [`TreasuryCapReclaimer<${tokenType}>`]);
        contracts[symbol.toUpperCase()].objects.TreasuryCapReclaimer = treasuryCapReclaimerId;
    }
}

async function removeUnlinkedCoin(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { InterchainTokenService } = itsConfig.objects;
    const walletAddress = keypair.toSuiAddress();
    const txBuilder = new TxBuilder(client);

    const symbol = args;
    validateParameters({
        isNonEmptyString: { symbol },
        isNonArrayObject: { tokenEntry: contracts[symbol.toUpperCase()] },
    });

    const coin = contracts[symbol.toUpperCase()];
    const tcrErrorMsg = `no TreasuryCapReclaimer was found for token with symbol ${symbol}`;
    if (!coin.objects) {
        throw new Error(tcrErrorMsg);
    } else if (!coin.objects.TreasuryCapReclaimer) {
        throw new Error(tcrErrorMsg);
    }

    // Receive TreasuryCap
    const treasuryCap = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::remove_unlinked_coin`,
        arguments: [InterchainTokenService, coin.objects.TreasuryCapReclaimer],
        typeArguments: [coin.typeArgument],
    });

    // Return TreasuryCap to coin deployer (TreasuryCapReclaimer owner)
    txBuilder.tx.transferObjects([treasuryCap], walletAddress);

    await broadcastFromTxBuilder(txBuilder, keypair, `Remove Unlinked Coin (${symbol})`, options);

    // Remove TreasuryCapReclaimer as it's been deleted
    contracts[symbol.toUpperCase()].objects.TreasuryCapReclaimer = null;
}

async function registerCoinMetadata(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig, AxelarGateway } = contracts;
    const { InterchainTokenService } = itsConfig.objects;
    const { Gateway } = AxelarGateway.objects;
    const [symbol] = args;

    validateParameters({
        isNonEmptyString: { symbol },
    });

    const walletAddress = keypair.toSuiAddress();
    const deployConfig = { client, keypair, options, walletAddress };

    const destinationChain = 'axelar';

    // If coin is already deployed load it, else deploy a new coin
    const savedCoin = contracts[symbol.toUpperCase()];
    if (!savedCoin && !options.coinName && !options.coinDecimals) {
        throw new Error(
            `Coin name and decimals are required for coins not saved in config, found: ${JSON.stringify([
                options.coinName,
                options.coinDecimals,
            ])}`,
        );
    }

    let metadata, packageId, tokenType, treasuryCap;
    if (!savedCoin) {
        // Deploy source token on Sui
        [metadata, packageId, tokenType, treasuryCap] = await deployTokenFromInfo(
            deployConfig,
            symbol,
            options.coinName,
            options.coinDecimals,
        );
    } else {
        // Load saved coin params
        metadata = savedCoin.objects.Metadata;
        packageId = savedCoin.address;
        tokenType = savedCoin.typeArgument;
        treasuryCap = savedCoin.objects.TreasuryCap;
    }

    // User calls registerTokenMetadata on ITS Chain A to submit a RegisterTokenMetadata msg type to
    // ITS Hub to register token data in ITS hub.
    const txBuilder = new TxBuilder(client);

    const messageTicket = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::register_coin_metadata`,
        arguments: [InterchainTokenService, metadata],
        typeArguments: [tokenType],
    });

    // Pay gas for register coin metadata cross-chain message
    const { gasFeeValue } = await estimateITSFee(
        config.chains[options.chainName],
        destinationChain,
        options.env,
        'TokenMetadataRegistered',
        'auto',
        config.axelar,
    );

    const [gas] = txBuilder.tx.splitCoins(txBuilder.tx.gas, [gasFeeValue]);

    await txBuilder.moveCall({
        target: `${contracts.GasService.address}::gas_service::pay_gas`,
        typeArguments: [suiCoinId],
        arguments: [contracts.GasService.objects.GasService, messageTicket, gas, walletAddress, '0x'],
    });

    await txBuilder.moveCall({
        target: `${AxelarGateway.address}::gateway::send_message`,
        arguments: [Gateway, messageTicket],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, `Register Token Metadata (${symbol})`, options);

    if (!savedCoin) {
        // Save deployed tokens
        saveTokenDeployment(
            packageId,
            tokenType,
            contracts,
            symbol,
            options.coinDecimals,
            null, // TokenId does not yet exist (pre-registration)
            treasuryCap,
            metadata,
        );
    }
}

async function linkCoin(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig, AxelarGateway } = contracts;
    const { InterchainTokenService } = itsConfig.objects;
    const { Gateway } = AxelarGateway.objects;
    const [symbol, destinationChain, destinationAddress] = args;

    const unvalidatedParams = {
        isNonEmptyString: { symbol, destinationChain, destinationAddress },
        isNonArrayObject: { tokenEntry: contracts[symbol.toUpperCase()] },
    };

    if (options.salt) {
        unvalidatedParams.isHexString = { salt: options.salt };
    }

    validateParameters(unvalidatedParams);

    const destinationTokenAddress = encodeITSDestinationToken(config.chains, destinationChain, destinationAddress);

    const walletAddress = keypair.toSuiAddress();
    const deployConfig = { client, keypair, options, walletAddress };

    // Coin params
    const coin = contracts[symbol.toUpperCase()];
    const decimals = coin.decimals;
    const metadata = coin.objects.Metadata;
    const packageId = coin.address;
    const tokenType = coin.typeArgument;
    const treasuryCap = coin.objects.TreasuryCap;

    // Token Manager settings
    const tokenManager = options.tokenManagerMode;
    const destinationTokenManager = options.destinationTokenManagerMode;

    // User calls registerCustomToken on ITS Chain A to register the token on the source chain.
    // A token manager is deployed on the source chain corresponding to the tokenId.
    let txSalt = options.salt ? options.salt : coin.saltAddress;
    let tokenId = coin.objects.TokenId ? coin.objects.TokenId : null;
    let channelId = options.channel ? options.channel : null;
    if (!options.registered) {
        const [token, channel, saltAddress] = await registerCustomCoinUtil(
            deployConfig,
            itsConfig,
            AxelarGateway,
            symbol,
            metadata,
            tokenType,
            tokenManager === 'mint_burn' ? treasuryCap : null, // Token manager type (souce chain)
            options.salt ? options.salt : null,
        );

        txSalt = saltAddress;
        tokenId = token;
        channelId = channel;
    } else {
        if (!txSalt) {
            throw new Error(`error resolving unique salt, got ${txSalt}`);
        }
    }

    // User then calls linkToken on ITS Chain A with the destination token address for Chain B.
    // This submits a LinkToken msg type to ITS Hub.
    const txBuilder = new TxBuilder(client);

    if (!channelId) {
        throw new Error(`error deriving channel that registered custom token ${tokenId}, got ${channelId}`);
    }

    // Token manager type (destination chain)
    const tokenManagerType = await txBuilder.moveCall({
        target: `${itsConfig.address}::token_manager_type::${destinationTokenManager}`,
    });

    // Salt
    const salt = await txBuilder.moveCall({
        target: `${AxelarGateway.address}::bytes32::new`,
        arguments: [txSalt],
    });

    const linkParams = options.destinationOperator
        ? encodeITSDestination(config.chains, destinationChain, options.destinationOperator)
        : '0x';

    const messageTicket = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::link_coin`,
        arguments: [
            InterchainTokenService,
            channelId,
            salt,
            destinationChain, // chain must be already added as a trusted chain
            destinationTokenAddress,
            tokenManagerType,
            linkParams,
        ],
    });

    // Pay gas for link coin cross-chain message
    const { gasFeeValue } = await estimateITSFee(
        config.chains[options.chainName],
        destinationChain,
        options.env,
        'LinkToken',
        'auto',
        config.axelar,
    );

    const [gas] = txBuilder.tx.splitCoins(txBuilder.tx.gas, [gasFeeValue]);

    await txBuilder.moveCall({
        target: `${contracts.GasService.address}::gas_service::pay_gas`,
        typeArguments: [suiCoinId],
        arguments: [contracts.GasService.objects.GasService, messageTicket, gas, walletAddress, '0x'],
    });

    await txBuilder.moveCall({
        target: `${AxelarGateway.address}::gateway::send_message`,
        arguments: [Gateway, messageTicket],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, `Link Coin (${symbol})`, options);

    // Linked tokens (source / destination)
    const sourceToken = { metadata, packageId, tokenType, treasuryCap };
    const linkedTokens = Array.isArray(coin.linkedTokens)
        ? [...coin.linkedTokens, { destinationChain, destinationAddress }]
        : [{ destinationChain, destinationAddress }];

    // Save deployed tokens
    saveTokenDeployment(
        sourceToken.packageId,
        sourceToken.tokenType,
        contracts,
        symbol,
        decimals,
        tokenId,
        sourceToken.treasuryCap,
        sourceToken.metadata,
        linkedTokens,
        txSalt,
        tokenManager,
    );
}

async function deployRemoteCoin(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const walletAddress = keypair.toSuiAddress();
    const txBuilder = new TxBuilder(client);

    const tx = txBuilder.tx;

    const [tokenId, destinationChain] = args;

    validateParameters({
        isHexString: { tokenId },
    });

    validateDestinationChain(config.chains, destinationChain);

    // Fetch CoinType from on-chain TokenID
    const coinType = await tokenIdToCoinType(client, walletAddress, itsConfig, tokenId);

    const tokenIdObj = await txBuilder.moveCall({
        target: `${itsConfig.address}::token_id::from_u256`,
        arguments: [tokenId],
    });

    const deployRemoteTokenTicket = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::deploy_remote_interchain_token`,
        arguments: [itsConfig.objects.InterchainTokenService, tokenIdObj, destinationChain],
        typeArguments: [coinType],
    });

    printInfo('🚀 Deploying remote interchain token....');

    const unitAmountGas = parseUnits('1', 9).toBigInt();

    const [gas] = tx.splitCoins(tx.gas, [unitAmountGas]);

    await txBuilder.moveCall({
        target: `${contracts.GasService.address}::gas_service::pay_gas`,
        typeArguments: [suiCoinId],
        arguments: [contracts.GasService.objects.GasService, deployRemoteTokenTicket, gas, walletAddress, '0x'],
    });

    await txBuilder.moveCall({
        target: `${contracts.AxelarGateway.address}::gateway::send_message`,
        arguments: [contracts.AxelarGateway.objects.Gateway, deployRemoteTokenTicket],
    });

    if (options.offline) {
        const tx = txBuilder.tx;
        const sender = options.sender || keypair.toSuiAddress();
        tx.setSender(sender);
        await saveGeneratedTx(tx, `✅ Remote coin deployment completed for ${tokenId}`, client, options);
    } else {
        await broadcastFromTxBuilder(txBuilder, keypair, 'Deploy remote coin', options);
    }
}

async function removeTreasuryCap(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { InterchainTokenService } = itsConfig.objects;
    const walletAddress = keypair.toSuiAddress();
    const txBuilder = new TxBuilder(client);

    const symbol = args;
    validateParameters({
        isNonEmptyString: { symbol },
        isNonArrayObject: { tokenEntry: contracts[symbol.toUpperCase()] },
    });

    const coin = contracts[symbol.toUpperCase()];
    const tcrErrorMsg = `no TreasuryCapReclaimer was found for token with symbol ${symbol}`;
    if (!coin.objects) {
        throw new Error(tcrErrorMsg);
    } else if (!coin.objects.TreasuryCapReclaimer) {
        throw new Error(tcrErrorMsg);
    }

    // Receive TreasuryCap
    const treasuryCap = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::remove_treasury_cap`,
        arguments: [InterchainTokenService, coin.objects.TreasuryCapReclaimer],
        typeArguments: [coin.typeArgument],
    });

    // Return TreasuryCap to coin deployer (TreasuryCapReclaimer owner)
    // coin will be unusable by ITS until `restore_treasury_cap` is called
    txBuilder.tx.transferObjects([treasuryCap], walletAddress);

    await broadcastFromTxBuilder(txBuilder, keypair, `Remove TreasuryCap (${symbol})`, options);

    // Remove TreasuryCapReclaimer as it's been deleted
    contracts[symbol.toUpperCase()].objects.TreasuryCapReclaimer = null;
}

async function restoreTreasuryCap(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { InterchainTokenService } = itsConfig.objects;
    const walletAddress = keypair.toSuiAddress();
    const txBuilder = new TxBuilder(client);

    const symbol = args;
    validateParameters({
        isNonEmptyString: { symbol },
        isNonArrayObject: { tokenEntry: contracts[symbol.toUpperCase()] },
    });

    const coin = contracts[symbol.toUpperCase()];
    const tcErrorMsg = `no TreasuryCap was found for token with symbol ${symbol}`;
    if (!coin.objects) {
        throw new Error(tcErrorMsg);
    } else if (!coin.objects.TreasuryCap) {
        throw new Error(tcErrorMsg);
    }

    // TokenId
    const tokenId = await txBuilder.moveCall({
        target: `${itsConfig.address}::token_id::from_address`,
        arguments: [coin.objects.TokenId],
    });

    // Receive TreasuryCapReclaimer
    const treasuryCapReclaimer = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::restore_treasury_cap`,
        arguments: [InterchainTokenService, coin.objects.TreasuryCap, tokenId],
        typeArguments: [coin.typeArgument],
    });

    // Return TreasuryCapReclaimer to coin deployer (TreasuryCap owner)
    // coin will be usable again by ITS
    txBuilder.tx.transferObjects([treasuryCapReclaimer], walletAddress);

    const result = await broadcastFromTxBuilder(txBuilder, keypair, `Restore TreasuryCap (${symbol})`, options);

    // Save TreasuryCapReclaimer to coin config
    const [treasuryCapReclaimerId] = getObjectIdsByObjectTypes(result, [`TreasuryCapReclaimer<${coin.typeArgument}>`]);
    contracts[symbol.toUpperCase()].objects.TreasuryCapReclaimer = treasuryCapReclaimerId;
}

async function interchainTransfer(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const [tokenId, destinationChain, destinationAddress, amount] = args;
    const walletAddress = keypair.toSuiAddress();

    validateParameters({
        isHexString: { tokenId },
        isValidNumber: { amount },
    });

    validateDestinationChain(config.chains, destinationChain);

    // Fetch CoinType from on-chain TokenID
    const coinType = await tokenIdToCoinType(client, walletAddress, itsConfig, tokenId);

    let coinPackageId, coinDecimals;
    try {
        const coinMetadata = await client.getCoinMetadata({ coinType });
        coinDecimals = coinMetadata.decimals;
        coinPackageId = coinType.split('::')[0];
    } catch {
        throw new Error(`Error parsing coin metadata for coin ${coinType}`);
    }

    const txBuilder = new TxBuilder(client);
    const tx = txBuilder.tx;

    const tokenIdObj = await txBuilder.moveCall({
        target: `${itsConfig.address}::token_id::from_u256`,
        arguments: [tokenId],
    });

    const gatewayChannelId = options.channel
        ? options.channel
        : await txBuilder.moveCall({
              target: `${contracts.AxelarGateway.address}::channel::new`,
              arguments: [],
          });

    // Coin must exist
    await checkIfCoinExists(client, coinPackageId, coinType);

    // Convert human readable coin amount to send value
    const unitAmount = getUnitAmount(amount, coinDecimals);

    // Check balance and load valid coin id
    const { coinObjectId, balance } = await senderHasSufficientBalance(client, keypair, options, coinType, unitAmount);

    // Split coins (if required)
    let coinsToSend;
    if (parseInt(balance) === parseInt(unitAmount)) {
        coinsToSend = coinObjectId;
    } else {
        [coinsToSend] = tx.splitCoins(coinObjectId, [unitAmount]);
    }

    // Interchain transfer
    const prepareInterchainTransferTicket = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::prepare_interchain_transfer`,
        typeArguments: [coinType],
        arguments: [tokenIdObj, coinsToSend, destinationChain, destinationAddress, '0x', gatewayChannelId],
    });

    const interchainTransferTicket = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::send_interchain_transfer`,
        typeArguments: [coinType],
        arguments: [itsConfig.objects.InterchainTokenService, prepareInterchainTransferTicket, suiClockAddress],
    });

    const { gasFeeValue } = await estimateITSFee(
        config.chains[options.chainName],
        destinationChain,
        options.env,
        'InterchainTransfer',
        'auto',
        config.axelar,
    );

    const [gas] = tx.splitCoins(tx.gas, [gasFeeValue]);

    await txBuilder.moveCall({
        target: `${contracts.GasService.address}::gas_service::pay_gas`,
        typeArguments: [suiCoinId],
        arguments: [contracts.GasService.objects.GasService, interchainTransferTicket, gas, walletAddress, '0x'],
    });

    await txBuilder.moveCall({
        target: `${contracts.AxelarGateway.address}::gateway::send_message`,
        arguments: [contracts.AxelarGateway.objects.Gateway, interchainTransferTicket],
    });

    // If a temp channel was created, destroy it
    if (!options.channel) {
        await txBuilder.moveCall({
            target: `${contracts.AxelarGateway.address}::channel::destroy`,
            arguments: [gatewayChannelId],
        });
    }

    if (options.offline) {
        const tx = txBuilder.tx;
        const sender = options.sender || keypair.toSuiAddress();
        tx.setSender(sender);
        await saveGeneratedTx(tx, `Interchain transfer for ${tokenId}`, client, options);
    } else {
        await broadcastFromTxBuilder(txBuilder, keypair, 'Interchain Transfer', options);
    }
}

async function checkVersionControl(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const version = args;

    validateParameters({
        isNonEmptyString: {
            version,
            versionEntry: itsConfig.versions[version],
            itsObject: itsConfig.objects.InterchainTokenServicev0,
        },
    });

    const supportedFunctions = itsFunctions[version];
    if (!Array.isArray(supportedFunctions)) {
        throw new Error(`No deployable versions found with id ${version}`);
    }

    const versionedId = itsConfig.objects.InterchainTokenServicev0;
    const allowedFunctionsArray = await getAllowedFunctions(client, versionedId);
    const allowedFunctions = allowedFunctionsArray[parseInt(version)];
    const equality = JSON.stringify(allowedFunctions) === JSON.stringify(supportedFunctions);

    if (equality) {
        printInfo(`All functions are allowed in version ${version}`, allowedFunctions);
    } else {
        const disabledFunctions = [];
        const enabledFunctions = [];
        supportedFunctions.forEach((fnName) => {
            if (allowedFunctions.indexOf(fnName) > -1) {
                enabledFunctions.push(fnName);
            } else {
                disabledFunctions.push(fnName);
            }
        });

        printInfo(`${enabledFunctions.length} functions are allowed in version`, version);
        printInfo(`${disabledFunctions.length} functions are not allowed in version`, version);
        printInfo('Allowed functions', allowedFunctions);
        printInfo('Disallowed functions', disabledFunctions);
    }
}

async function processCommand(command, config, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    await command(keypair, client, config, chain.contracts, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(command, config, chain, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('InterchainTokenService').description('SUI InterchainTokenService scripts');

    // This command is used to setup the trusted chains on the InterchainTokenService contract.
    // The trusted chain is used to verify the message from the source chain.
    const addTrustedChainsProgram = new Command()
        .name('add-trusted-chains')
        .command('add-trusted-chains <trusted-chains...>')
        .description(
            `Add trusted chains. The <trusted-chains> can be a list of chains separated by whitespaces. It can also be a special tag to indicate a specific set of chains e.g. 'all' to target all InterchainTokenService-deployed chains.`,
        )
        .action((trustedChains, options) => {
            mainProcessor(addTrustedChains, options, trustedChains, processCommand);
        });

    const removeTrustedChainsProgram = new Command()
        .name('remove-trusted-chains')
        .description('Remove trusted chains.')
        .command('remove-trusted-chains <trusted-chains...>')
        .action((trustedChains, options) => {
            mainProcessor(removeTrustedChains, options, trustedChains, processCommand);
        });

    const setFlowLimitsProgram = new Command()
        .name('set-flow-limits')
        .command('set-flow-limits <token-ids> <flow-limits>')
        .description(`Set flow limits for multiple tokens. <token-ids> and <flow-limits> can both be comma separated lists.`)
        .action((tokenIds, flowLimits, options) => {
            mainProcessor(setFlowLimits, options, [tokenIds, flowLimits], processCommand);
        });

    const registerCoinFromInfoProgram = new Command()
        .name('register-coin-from-info')
        .command('register-coin-from-info <symbol> <name> <decimals>')
        .description(`Deploy a coin on SUI and register it in ITS using token name, symbol and decimals.`)
        .action((symbol, name, decimals, options) => {
            mainProcessor(registerCoinFromInfo, options, [symbol, name, decimals], processCommand);
        });

    const registerCoinFromMetadataProgram = new Command()
        .name('register-coin-from-metadata')
        .command('register-coin-from-metadata <symbol> <name> <decimals>')
        .description(`Deploy a coin on SUI and register it in ITS using its coin metadata.`)
        .action((symbol, name, decimals, options) => {
            mainProcessor(registerCoinFromMetadata, options, [symbol, name, decimals], processCommand);
        });

    const registerCustomCoinProgram = new Command()
        .name('register-custom-coin')
        .command('register-custom-coin <symbol> <name> <decimals>')
        .description(
            `Register a custom coin in ITS using token name, symbol and decimals. If no salt is provided, it will be automatically created.`,
        )
        .addOption(new Option('--channel <channel>', 'Existing channel ID to initiate a cross-chain message over'))
        .addOption(new Option('--treasuryCap', `Give the coin's TreasuryCap to ITS`))
        .addOption(new Option('--salt <salt>', 'An address in hexidecimal to be used as salt in the Token ID'))
        .addOption(new Option('--mintAmount <amount>', 'Amount of pre-registration tokens to mint to the deployer').default('1000'))
        .addOption(new Option('--published', 'Skip token deployment and only do coin registration'))
        .action((symbol, name, decimals, options) => {
            mainProcessor(registerCustomCoin, options, [symbol, name, decimals], processCommand);
        });

    const migrateCoinMetadataProgram = new Command()
        .name('migrate-coin-metadata')
        .command('migrate-coin-metadata <symbol>')
        .description(`Release metadata for a single token saved in the chain config and migrate it to a publicly shared object.`)
        .action((symbol, options) => {
            mainProcessor(migrateCoinMetadata, options, symbol, processCommand);
        });

    const migrateAllCoinMetadataProgram = new Command()
        .name('migrate-coin-metadata-all')
        .command('migrate-coin-metadata-all')
        .description(`Release metadata for all legacy coins saved to the chain config (see command: its/tokens legacy-coins).`)
        .addOption(
            new Option(
                '--logging <size>',
                'Print a status update every <size> of migrated tokens, or use <size> 0 to disable logging. Defaults to 0.',
            ),
        )
        .addOption(
            new Option(
                '--batch <size>',
                'Process migrations in a batch of <size> transactions, or use <size> 0 for no batching. Defaults to 0.',
            ),
        )
        .action((options) => {
            mainProcessor(migrateAllCoinMetadata, options, null, processCommand);
        });

    const giveUnlinkedCoinProgram = new Command()
        .name('give-unlinked-coin')
        .command('give-unlinked-coin <symbol> <tokenId>')
        .description(`Call give unlinked coin and give its treasury capability to ITS.`)
        .addOption(new Option('--treasuryCapReclaimer', 'Pass this flag to retain the ability to reclaim the treasury capability'))
        .action((symbol, tokenId, options) => {
            mainProcessor(giveUnlinkedCoin, options, [symbol, tokenId], processCommand);
        });

    const removeUnlinkedCoinProgram = new Command()
        .name('remove-unlinked-coin')
        .command('remove-unlinked-coin <symbol>')
        .description(`Remove a coin from ITS and return its TreasuryCap to its deployer.`)
        .action((symbol, options) => {
            mainProcessor(removeUnlinkedCoin, options, symbol, processCommand);
        });

    const registerCoinMetadataProgram = new Command()
        .name('register-coin-metadata')
        .command('register-coin-metadata <symbol>')
        .description(`Load or deploy a source coin on SUI using its symbol, and register its metadata on Axelar Hub.`)
        .addOption(new Option('--coinName <name>', 'Optional coin name (mandatory if coin not saved in config)'))
        .addOption(new Option('--coinDecimals <decimals>', 'Optional coin decimals (mandatory if coin not saved in config)'))
        .action((symbol, options) => {
            mainProcessor(registerCoinMetadata, options, [symbol], processCommand);
        });

    const linkCoinProgram = new Command()
        .name('link-coin')
        .command('link-coin <symbol> <destinationChain> <destinationAddress>')
        .description(
            `Link a coin with the destination using the destination chain name and address. Token metadata must be registered on Axelar Hub.`,
        )
        .addOption(new Option('--channel <channel>', 'Existing channel ID to initiate a cross-chain message over'))
        .addOption(
            new Option('--tokenManagerMode <mode>', 'Token Manager Mode').choices(['lock_unlock', 'mint_burn']).default('lock_unlock'),
        )
        .addOption(
            new Option('--destinationTokenManagerMode <mode>', ' Destination Token Manager Mode')
                .choices(['lock_unlock', 'mint_burn'])
                .makeOptionMandatory(true),
        )
        .addOption(
            new Option(
                '--destinationOperator <operator>',
                'Optional token manager address on the destination chain. If provided, used as link paramater.',
            ),
        )
        .addOption(new Option('--salt <salt>', 'An address in hexidecimal to be used as salt in the Token ID'))
        .addOption(new Option('--registered', 'Skip token registration and only do coin linking'))
        .action((symbol, destinationChain, destinationAddress, options) => {
            mainProcessor(linkCoin, options, [symbol, destinationChain, destinationAddress], processCommand);
        });

    const deployRemoteCoinProgram = new Command()
        .name('deploy-remote-coin')
        .command('deploy-remote-coin <tokenId> <destinationChain>')
        .description(`Deploy an interchain token on a remote chain`)
        .action((tokenId, destinationChain, options) => {
            mainProcessor(deployRemoteCoin, options, [tokenId, destinationChain], processCommand);
        });

    const removeTreasuryCapProgram = new Command()
        .name('remove-treasury-cap')
        .command('remove-treasury-cap <symbol>')
        .description(`Transfer a coin's TreasuryCap to the deployer to reclaim mint/burn permission from ITS.`)
        .action((symbol, options) => {
            mainProcessor(removeTreasuryCap, options, symbol, processCommand);
        });

    const restoreTreasuryCapProgram = new Command()
        .name('restore-treasury-cap')
        .command('restore-treasury-cap <symbol>')
        .description(`Restore a coin's TreasuryCap to ITS after calling remove-treasury-cap, giving mint/burn permission back to ITS.`)
        .action((symbol, options) => {
            mainProcessor(restoreTreasuryCap, options, symbol, processCommand);
        });

    const checkVersionControlProgram = new Command()
        .name('check-version-control')
        .command('check-version-control <version>')
        .description('Check if version control works on a certain version.')
        .action((version, options) => {
            mainProcessor(checkVersionControl, options, version, processCommand);
        });

    const interchainTransferProgram = new Command()
        .name('interchain-transfer')
        .command('interchain-transfer <tokenId> <destinationChain> <destinationAddress> <amount>')
        .description('Send interchain transfer from sui to a chain where token is linked')
        .addOption(new Option('--channel <channel>', 'Existing channel ID to initiate a cross-chain message over'))
        .action((tokenId, destinationChain, destinationAddress, amount, options) => {
            mainProcessor(interchainTransfer, options, [tokenId, destinationChain, destinationAddress, amount], processCommand);
        });

    const listTrustedChainsProgram = new Command()
        .name('list-trusted-chains')
        .command('list-trusted-chains')
        .description('List the trusted chains configured in InterchainTokenService')
        .action((options) => {
            mainProcessor(listTrustedChains, options, null, processCommand);
        });

    program.addCommand(addTrustedChainsProgram);
    program.addCommand(checkVersionControlProgram);
    program.addCommand(deployRemoteCoinProgram);
    program.addCommand(giveUnlinkedCoinProgram);
    program.addCommand(interchainTransferProgram);
    program.addCommand(linkCoinProgram);
    program.addCommand(listTrustedChainsProgram);
    program.addCommand(migrateAllCoinMetadataProgram);
    program.addCommand(migrateCoinMetadataProgram);
    program.addCommand(registerCoinFromInfoProgram);
    program.addCommand(registerCoinFromMetadataProgram);
    program.addCommand(registerCustomCoinProgram);
    program.addCommand(removeUnlinkedCoinProgram);
    program.addCommand(registerCoinMetadataProgram);
    program.addCommand(removeTreasuryCapProgram);
    program.addCommand(removeTrustedChainsProgram);
    program.addCommand(restoreTreasuryCapProgram);
    program.addCommand(setFlowLimitsProgram);

    // finalize program
    addOptionsToCommands(program, addBaseOptions, { offline: true });
    program.parse();
}

module.exports = { addTrustedChains, removeTrustedChains, setFlowLimits };
