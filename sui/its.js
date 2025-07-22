const { Option, Command } = require('commander');
const { STD_PACKAGE_ID, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, getChainConfig, parseTrustedChains } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    broadcastFromTxBuilder,
    createSaltAddress,
    deployTokenFromInfo,
    getWallet,
    newCoinManagementLocked,
    printWalletInfo,
    registerCustomCoinUtil,
    saveGeneratedTx,
    saveTokenDeployment,
} = require('./utils');
const { bcs } = require('@mysten/sui/bcs');

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

    const trustedChains = parseTrustedChains(config, args);

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
    const [txBuilder, coinManagement] = await newCoinManagementLocked(deployConfig, itsConfig, tokenType);

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
    const [txBuilder, coinManagement] = await newCoinManagementLocked(deployConfig, itsConfig, tokenType);

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
    const [tokenId, _channelId, _salt] = await registerCustomCoinUtil(deployConfig, itsConfig, AxelarGateway, symbol, metadata, tokenType);
    if (!tokenId) throw new Error(`error resolving token id from registration tx, got ${tokenId}`);

    // Save the deployed token
    saveTokenDeployment(packageId, tokenType, contracts, symbol, decimals, tokenId, treasuryCap, metadata);
}

// migrate_coin_metadata
async function migrateCoinMetadata(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { OperatorCap, InterchainTokenService } = itsConfig.objects;
    const txBuilder = new TxBuilder(client);

    const symbol = args;
    if (!symbol) throw new Error('token symbol is required');
    if (!contracts[symbol.toUpperCase()]) throw new Error('token not found in deployments history');

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

    let txBuilder = new TxBuilder(client);

    // Deploy token on Sui
    const [metadata, packageId, tokenType, treasuryCap] = await deployTokenFromInfo(deployConfig, symbol, name, decimals);

    // Register deployed token (custom)
    const [tokenId, _channelId, _salt] = await registerCustomCoinUtil(deployConfig, itsConfig, AxelarGateway, symbol, metadata, tokenType);
    if (!tokenId) throw new Error(`error resolving token id from registration tx, got ${tokenId}`);

    // Option<TreasuryCap<T>> 
    const target = options.treasuryCap 
        ? `${STD_PACKAGE_ID}::option::some` 
        : `${STD_PACKAGE_ID}::option::none`;
    const arguments = options.treasuryCap ? [] : [treasuryCap];
    const typeArguments = [[itsConfig.structs.TreasuryCapReclaimer, '<', tokenType, '>'].join('')];
    const treasuryCapOption = await txBuilder.moveCall({ target, arguments, typeArguments });

    // give_unlinked_coin<T>
    const treasuryCapReclaimer = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::give_unlinked_coin`,
        arguments: [
            InterchainTokenService,
            tokenId,
            metadata,
            treasuryCapOption,
        ],
        typeArguments: [tokenType],
    });

    if (options.treasuryCap) txBuilder.tx.transferObjects([treasuryCapReclaimer], walletAddress);

    // Save the deployed token
    saveTokenDeployment(packageId, tokenType, contracts, symbol, decimals, tokenId, treasuryCap, metadata);    
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
    );
}

async function processCommand(command, config, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    await command(keypair, client, config, chain.contracts, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
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
            `Add trusted chains. The <trusted-chains> can be a list of chains separated by whitespaces. It can also be a special tag to indicate a specific set of chains e.g. 'all' to target all InterchainTokenService-deployed chains`,
        )
        .action((trustedChains, options) => {
            mainProcessor(addTrustedChains, options, trustedChains, processCommand);
        });

    const removeTrustedChainsProgram = new Command()
        .name('remove-trusted-chains')
        .description('Remove trusted chains')
        .command('remove-trusted-chains <trusted-chains...>')
        .action((trustedChains, options) => {
            mainProcessor(removeTrustedChains, options, trustedChains, processCommand);
        });

    const setFlowLimitsProgram = new Command()
        .name('set-flow-limits')
        .command('set-flow-limits <token-ids> <flow-limits>')
        .description(`Set flow limits for multiple tokens. <token-ids> and <flow-limits> can both be comma separated lists`)
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
        .action((symbol, name, decimals, options) => {
            mainProcessor(registerCustomCoin, options, [symbol, name, decimals], processCommand);
        });

    const migrateCoinMetadataProgram = new Command()
        .name('migrate-coin-metadata')
        .command('migrate-coin-metadata <symbol>')
        .description(`Release metadata for a given token id, can migrate tokens with metadata saved in ITS to v1`)
        .action((symbol, options) => {
            mainProcessor(migrateCoinMetadata, options, symbol, processCommand);
        });

    const giveUnlinkedCoinProgram = new Command()
        .name('give-unlinked-coin')
        .command('give-unlinked-coin <symbol> <name> <decimals>')
        .description(
            `Deploy a coin on Sui, register it as custom coin and give its treasury capability to ITS.`,
        )
        .addOption(new Option('--treasuryCapReclaimer', 'Pass this flag to retain the ability to reclaim the treasury capability'))
        .action((symbol, name, decimals, options) => {
            mainProcessor(giveUnlinkedCoin, options, [symbol, name, decimals], processCommand);
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

    // v0
    program.addCommand(setFlowLimitsProgram);
    program.addCommand(addTrustedChainsProgram);
    program.addCommand(removeTrustedChainsProgram);

    // v1
    program.addCommand(registerCoinFromInfoProgram);
    program.addCommand(registerCoinFromMetadataProgram);
    program.addCommand(registerCustomCoinProgram);
    program.addCommand(migrateCoinMetadataProgram);
    program.addCommand(giveUnlinkedCoinProgram);
    program.addCommand(linkCoinProgram);

    // finalize program
    addOptionsToCommands(program, addBaseOptions, { offline: true });
    program.parse();
}
