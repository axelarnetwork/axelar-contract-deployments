const { Option, Command } = require('commander');
const { STD_PACKAGE_ID, SUI_PACKAGE_ID, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, printInfo, saveConfig, getChainConfig, parseTrustedChains, validateParameters, isNonArrayObject, isNonEmptyString } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    broadcastFromTxBuilder,
    createLockedCoinManagement,
    deployTokenFromInfo,
    getAllowedFunctions,
    getObjectIdsByObjectTypes,
    getWallet,
    itsFunctions,
    printWalletInfo,
    registerCustomCoinUtil,
    saveGeneratedTx,
    saveTokenDeployment,
} = require('./utils');
const { bcs } = require('@mysten/sui/bcs');
const chalk = require('chalk');

async function setFlowLimits(keypair, client, config, contracts, args, options) {
    let [tokenIds, flowLimits] = args;

    const { InterchainTokenService: itsConfig } = contracts;

    const { OperatorCap, InterchainTokenService } = itsConfig.objects;

    const txBuilder = new TxBuilder(client);

    tokenIds = tokenIds.split(',');
    flowLimits = flowLimits.split(',');

    if (tokenIds.length !== flowLimits.length) throw new Error('<token-ids> and <flow-limits> have to have the same length.');

    for (const i in tokenIds) {
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

    const txBuilder = new TxBuilder(client);

    const trustedChains = parseTrustedChains(config.chains, args);

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

    if (trustedChains.length === 0) throw new Error('No chains names provided');

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

// register_coin_from_info
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

// register_coin_from_metadata
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

// register_custom_coin
async function registerCustomCoin(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig, AxelarGateway } = contracts;
    const walletAddress = keypair.toSuiAddress();
    const deployConfig = { client, keypair, options, walletAddress };
    const [symbol, name, decimals] = args;

    // Deploy token on Sui
    const [metadata, packageId, tokenType, treasuryCap] = await deployTokenFromInfo(deployConfig, symbol, name, decimals);

    // Register deployed token (custom)
    const [tokenId, _channelId, saltAddress, result] = await registerCustomCoinUtil(
        deployConfig,
        itsConfig,
        AxelarGateway,
        symbol,
        metadata,
        tokenType,
        options.treasuryCap ? treasuryCap : false,
    );
    if (!tokenId) throw new Error(`error resolving token id from registration tx, got ${tokenId}`);

    // Save the deployed token
    saveTokenDeployment(packageId, tokenType, contracts, symbol, decimals, tokenId, treasuryCap, metadata, [], saltAddress);

    // Save TreasuryCapReclaimer to coin config (if exists)
    if (options.treasuryCap && contracts[symbol.toUpperCase()]) {
        const [treasuryCapReclaimerId] = getObjectIdsByObjectTypes(result, [`TreasuryCapReclaimer<${tokenType}>`]);
        contracts[symbol.toUpperCase()].objects.TreasuryCapReclaimer = treasuryCapReclaimerId;
    }
}

// migrate_coin_metadata (all)
async function migrateAllCoinMetadata(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { OperatorCap, InterchainTokenService } = itsConfig.objects;

    // Show or disable logging output (0 = logging disabled)
    let logSize = options.logging ? parseInt(options.logging) : 0;
    if (isNaN(logSize)) logSize = 0;

    // Batch or process txs 1-by-1 (0 = batching disabled)
    let batchSize = options.batch ? parseInt(options.batch) : 0;
    if (isNaN(batchSize)) batchSize = 0;

    // Migrate all the coins. This might take a while.
    const legacyCoins = contracts.InterchainTokenService.legacyCoins ? contracts.InterchainTokenService.legacyCoins : [];

    if (!legacyCoins.length) printInfo('Warning: no migratable tokens were found in chain config for env', options.env, chalk.yellow);

    let migratedCoins = [],
        failedMigrations = [],
        currentBatch = [],
        processedBatches = 0,
        txBuilder = new TxBuilder(client);
    for (let i = 0; i < legacyCoins.length; i++) {
        const coin = legacyCoins[i];

        if (batchSize) currentBatch.push(coin);

        try {
            await txBuilder.moveCall({
                target: `${itsConfig.address}::interchain_token_service::migrate_coin_metadata`,
                arguments: [InterchainTokenService, OperatorCap, coin.TokenId],
                typeArguments: [coin.TokenType],
            });
            // Process tx as batch or indidivual migration (depending on options.batch)
            if (!batchSize || i == legacyCoins.length - 1 || (i + 1) % batchSize === 0) {
                // Broadcast batch / individual tx, and reset builder
                const txType = !batchSize ? coin.symbol : 'batched';
                await broadcastFromTxBuilder(txBuilder, keypair, `Migrate Coin Metadata (${txType})`, options);
                txBuilder = new TxBuilder(client);
                if (!batchSize) migratedCoins.push(coin);
                else {
                    migratedCoins = [...migratedCoins, ...currentBatch];
                    ++processedBatches;
                    currentBatch = [];
                }
            }
        } catch (e) {
            txBuilder = new TxBuilder(client);
            if (!batchSize) {
                printInfo(`Migrate metadata failed for coin ${coin.symbol}`, e, chalk.red);
                failedMigrations.push(coin);
            } else {
                printInfo(`Migrate metadata failed for batch ${processedBatches}`, e, chalk.red);
                currentBatch.push(coin);
                failedMigrations = [...failedMigrations, ...currentBatch];
                ++processedBatches;
                currentBatch = [];
            }
        }

        // Intermediate status debugging report (e.g. if options.logging enabled)
        if (logSize > 0 && (i + 1) % logSize === 0 && !batchSize)
            printInfo(`Migrated metadata for ${migratedCoins.length} tokens. Last migrated token`, coin.symbol);
        else if (logSize > 0 && (i + 1) % logSize === 0 && batchSize)
            printInfo(`Migrated metadata for ${migratedCoins.length} tokens. Processed batches`, processedBatches);
    }

    // Final status report
    if (migratedCoins.length) printInfo('Total coins migrated', migratedCoins.length);
    else printInfo('No coins were migrated');

    // Clean up chain config
    if (failedMigrations.length) {
        contracts.InterchainTokenService.legacyCoins = failedMigrations;
        printInfo('Number of failed migrations', failedMigrations.length, chalk.yellow);
    } else delete contracts.InterchainTokenService.legacyCoins;
}

// migrate_coin_metadata (single)
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

// give_unlinked_coin
async function giveUnlinkedCoin(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig, AxelarGateway } = contracts;
    const { InterchainTokenService } = itsConfig.objects;
    const walletAddress = keypair.toSuiAddress();
    const deployConfig = { client, keypair, options, walletAddress };
    const [symbol, name, decimals] = args;
    const txBuilder = new TxBuilder(client);

    // Deploy token on Sui
    const [metadata, packageId, tokenType, treasuryCap] = await deployTokenFromInfo(deployConfig, symbol, name, decimals);

    // Register deployed token (custom)
    const [tokenId, _channelId, saltAddress, _result] = await registerCustomCoinUtil(
        deployConfig,
        itsConfig,
        AxelarGateway,
        symbol,
        metadata,
        tokenType,
    );
    if (!tokenId) throw new Error(`error resolving token id from registration tx, got ${tokenId}`);

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
    const treasuryCapReclaimerOption = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::give_unlinked_coin`,
        arguments: [InterchainTokenService, tokenIdObject, metadata, treasuryCapOption],
        typeArguments: [tokenType],
    });

    // TreasuryCapReclaimer<T>
    const treasuryCapReclaimerType = [itsConfig.structs.TreasuryCapReclaimer, '<', tokenType, '>'].join('');
    if (options.treasuryCapReclaimer) {
        const treasuryCapReclaimer = await txBuilder.moveCall({
            target: `${STD_PACKAGE_ID}::option::extract`,
            arguments: [treasuryCapReclaimerOption],
            typeArguments: [treasuryCapReclaimerType],
        });

        txBuilder.tx.transferObjects([treasuryCapReclaimer], walletAddress);
    }

    await txBuilder.moveCall({
        target: `${STD_PACKAGE_ID}::option::destroy_none`,
        arguments: [treasuryCapReclaimerOption],
        typeArguments: [treasuryCapReclaimerType],
    });

    const result = await broadcastFromTxBuilder(txBuilder, keypair, `Give Unlinked Coin (${symbol})`, options);

    // Save the deployed token
    saveTokenDeployment(packageId, tokenType, contracts, symbol, decimals, tokenId, treasuryCap, metadata, [], saltAddress);

    // Save TreasuryCapReclaimer to coin config (if exists)
    if (options.treasuryCapReclaimer && contracts[symbol.toUpperCase()]) {
        const [treasuryCapReclaimerId] = getObjectIdsByObjectTypes(result, [`TreasuryCapReclaimer<${tokenType}>`]);
        contracts[symbol.toUpperCase()].objects.TreasuryCapReclaimer = treasuryCapReclaimerId;
    }
}

// remove_unlinked_coin
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
    if (!coin.objects) throw new Error(tcrErrorMsg);
    else if (!coin.objects.TreasuryCapReclaimer) throw new Error(tcrErrorMsg);

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

// link_coin
async function linkCoin(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig, AxelarGateway } = contracts;
    const { InterchainTokenService } = itsConfig.objects;
    const { Gateway } = AxelarGateway.objects;
    const [symbol, name, decimals, destinationChain, destinationAddress] = args;

    const walletAddress = keypair.toSuiAddress();
    const deployConfig = { client, keypair, options, walletAddress };

    // Deploy source token on Sui (Token A)
    const [metadata, packageId, tokenType, treasuryCap] = await deployTokenFromInfo(deployConfig, symbol, name, decimals);

    // User calls registerTokenMetadata on ITS Chain A to submit a RegisterTokenMetadata msg type to
    // ITS Hub to register token data in ITS hub.
    let txBuilder = new TxBuilder(client);

    let messageTicket = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::register_coin_metadata`,
        arguments: [InterchainTokenService, metadata],
        typeArguments: [tokenType],
    });

    await txBuilder.moveCall({
        target: `${AxelarGateway.address}::gateway::send_message`,
        arguments: [Gateway, messageTicket],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, `Register Token Metadata (${symbol})`, options);

    // User calls registerCustomToken on ITS Chain A to register the token on the source chain.
    // A token manager is deployed on the source chain corresponding to the tokenId.
    const [tokenId, channelId, saltAddress] = await registerCustomCoinUtil(
        deployConfig,
        itsConfig,
        AxelarGateway,
        symbol,
        metadata,
        tokenType,
    );

    if (!tokenId) throw new Error(`error resolving token id from registration tx, got ${tokenId}`);
    if (!options.channel && !channelId) throw new Error(`error resolving channel id from registration tx, got ${channelId}`);

    const channel = options.channel ? options.channel : channelId;

    // User then calls linkToken on ITS Chain A with the destination token address for Chain B.
    // This submits a LinkToken msg type to ITS Hub.
    txBuilder = new TxBuilder(client);

    const tokenManagerType = await txBuilder.moveCall({
        target: `${itsConfig.address}::token_manager_type::lock_unlock`,
    });

    // Salt
    const salt = await txBuilder.moveCall({
        target: `${AxelarGateway.address}::bytes32::new`,
        arguments: [saltAddress],
    });

    messageTicket = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::link_coin`,
        arguments: [
            InterchainTokenService,
            channel,
            salt,
            destinationChain, // This assumes the chain is already added as a trusted chain
            bcs.string().serialize(destinationAddress).toBytes(),
            tokenManagerType,
            bcs.string().serialize('link params').toBytes(), // TODO: what value should go here?
        ],
    });

    await txBuilder.moveCall({
        target: `${AxelarGateway.address}::gateway::send_message`,
        arguments: [Gateway, messageTicket],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, `Link Coin (${symbol})`, options);

    // Linked tokens (source / destination)
    const sourceToken = { metadata, packageId, tokenType, treasuryCap };
    const linkedToken = { destinationChain, destinationAddress };

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
        [linkedToken],
        saltAddress,
    );
}

// remove_treasury_cap
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
    if (!coin.objects) throw new Error(tcrErrorMsg);
    else if (!coin.objects.TreasuryCapReclaimer) throw new Error(tcrErrorMsg);

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

// restore_treasury_cap
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
    if (!coin.objects) throw new Error(tcErrorMsg);
    else if (!coin.objects.TreasuryCap) throw new Error(tcErrorMsg);

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

//here
async function checkVersionControl(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const version = args;

    validateParameters({
        isNonEmptyString: { version },
        isNonEmptyString: { versionEntry: itsConfig.versions[version] },
        isNonEmptyString: { itsObject: itsConfig.objects.InterchainTokenServicev0 },
    });

    const supportedFunctions = itsFunctions[version];
    if (!Array.isArray(supportedFunctions)) throw new Error(`No deployable versions found with id ${version}`);

    const versionedId = itsConfig.objects.InterchainTokenServicev0;
    const allowedFunctionsArray = await getAllowedFunctions(client, versionedId);

    console.log(allowedFunctionsArray);
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

    // v0 release

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

    // v1 release
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
        .description(`Register a custom coin in ITS using token name, symbol and decimals. Salt is automatically created.`)
        .addOption(new Option('--channel <channel>', 'Existing channel ID to initiate a cross-chain message over'))
        .addOption(new Option('--treasuryCap', `Give the coin's TreasuryCap to ITS`))
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
        .command('give-unlinked-coin <symbol> <name> <decimals>')
        .description(`Deploy a coin on Sui, register it as custom coin and give its treasury capability to ITS.`)
        .addOption(new Option('--treasuryCapReclaimer', 'Pass this flag to retain the ability to reclaim the treasury capability'))
        .action((symbol, name, decimals, options) => {
            mainProcessor(giveUnlinkedCoin, options, [symbol, name, decimals], processCommand);
        });

    const removeUnlinkedCoinProgram = new Command()
        .name('remove-unlinked-coin')
        .command('remove-unlinked-coin <symbol>')
        .description(`Remove a coin from ITS and return its TreasuryCap to its deployer.`)
        .action((symbol, options) => {
            mainProcessor(removeUnlinkedCoin, options, symbol, processCommand);
        });

    const linkCoinProgram = new Command()
        .name('link-coin')
        .command('link-coin <symbol> <name> <decimals> <destinationChain> <destinationAddress>')
        .description(
            `Deploy a source coin on SUI and register it in ITS using custom registration, then link it with the destination using the destination chain name and address.`,
        )
        .addOption(new Option('--channel <channel>', 'Existing channel ID to initiate a cross-chain message over'))
        .action((symbol, name, decimals, destinationChain, destinationAddress, options) => {
            mainProcessor(linkCoin, options, [symbol, name, decimals, destinationChain, destinationAddress], processCommand);
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

    // v0
    program.addCommand(setFlowLimitsProgram);
    program.addCommand(addTrustedChainsProgram);
    program.addCommand(removeTrustedChainsProgram);

    // v1
    program.addCommand(registerCoinFromInfoProgram);
    program.addCommand(registerCoinFromMetadataProgram);
    program.addCommand(registerCustomCoinProgram);
    program.addCommand(migrateCoinMetadataProgram);
    program.addCommand(migrateAllCoinMetadataProgram);
    program.addCommand(giveUnlinkedCoinProgram);
    program.addCommand(removeUnlinkedCoinProgram);
    program.addCommand(linkCoinProgram);
    program.addCommand(removeTreasuryCapProgram);
    program.addCommand(restoreTreasuryCapProgram);
    program.addCommand(checkVersionControlProgram);

    // finalize program
    addOptionsToCommands(program, addBaseOptions, { offline: true });
    program.parse();
}
