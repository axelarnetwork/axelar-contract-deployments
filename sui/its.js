const { randomBytes } = require('node:crypto');
const { Command } = require('commander');
const { STD_PACKAGE_ID, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, getChainConfig, parseTrustedChains } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    broadcastFromTxBuilder,
    deployTokenFromInfo,
    getWallet,
    newCoinManagementLocked,
    printWalletInfo,
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

    // Deploy token on Sui
    const { metadata, packageId, tokenType, treasuryCap } = deployTokenFromInfo(symbol, name, decimals, walletAddress);

    // New CoinManagement<T>
    const coinManagement = newCoinManagementLocked(itsConfig, tokenType, walletAddress);

    // Register deployed token (from info)
    const txBuilder = new TxBuilder(client);
    const tokenId = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::register_coin_from_info`,
        arguments: [InterchainTokenService, name, symbol, decimals, coinManagement],
        typeArguments: [tokenType],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, `Register coin (${symbol}) from info in InterchainTokenService`, options, {
        showEvents: true,
    });

    // Save the deployed token info in the contracts object
    saveTokenDeployment(packageId, contracts, symbol, tokenId, treasuryCap, metadata);
}

// register_coin_from_metadata
// (XXX: covered in its-example.js#deployToken)

// register_custom_coin
async function registerCustomCoin(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { ChannelId, InterchainTokenService } = itsConfig.objects;
    const [symbol, name, decimals] = args;

    const walletAddress = keypair.toSuiAddress();

    // Deploy token on Sui
    const { metadata, packageId, tokenType, treasuryCap } = deployTokenFromInfo(symbol, name, decimals, walletAddress);

    // New CoinManagement<T>
    const coinManagement = newCoinManagementLocked(itsConfig, tokenType, walletAddress);

    // Register deployed token (from info)
    const salt = randomBytes(32);
    const txBuilder = new TxBuilder(client);
    const [tokenId] = await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::register_custom_coin`,
        arguments: [InterchainTokenService, ChannelId, salt, metadata, coinManagement],
        typeArguments: [tokenType],
    });

    // Save the deployed token info in the contracts object
    saveTokenDeployment(packageId, contracts, symbol, tokenId, treasuryCap, metadata);
}

// link_coin

// register_coin_metadata
// receive_link_coin
// give_unlinked_coin
// remove_treasury_cap
// restore_treasury_cap

// migrate_coin_metadata
async function migrateCoinMetadata(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { OperatorCap, InterchainTokenService } = itsConfig.objects;
    const txBuilder = new TxBuilder(client);

    const [tokenId, symbol] = args;
    if (!tokenId || !symbol) throw new Error('token_id and token_type are required');

    const tokenType = contracts[symbol.toUpperCase()].typeArgument;

    await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::migrate_coin_metadata`,
        arguments: [InterchainTokenService, OperatorCap, tokenId],
        typeArguments: [tokenType],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, 'Migrate Coin Metadata', options);
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

    // v0 release:

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

    // v1 release:

    const registerCoinFromInfoProgram = new Command()
        .name('register-coin-from-info')
        .command('register-coin-from-info <symbol> <name> <decimals>')
        .description(`Deploy a coin on SUI and register it in ITS using token name, symbol and decimals.`)
        .action((symbol, name, decimals, options) => {
            mainProcessor(registerCoinFromInfo, options, [symbol, name, decimals], processCommand);
        });
    
    const registerCustomCoinProgram = new Command()
        .name('register-custom-coin')
        .command('register-custom-coin <symbol> <name> <decimals>')
        .description(`Register a custom coin in ITS using token name, symbol and decimals. Salt is automatically created.`)
        .action((symbol, name, decimals, options) => {
            mainProcessor(registerCustomCoin, options, [symbol, name, decimals], processCommand);
        });

    const migrateCoinMetadataProgram = new Command()
        .name('migrate-coin-metadata')
        .command('migrate-coin-metadata <token-id> <token-symbol>')
        .description(`Release metadata for a given token id, can migrate tokens with metadata saved in ITS to v1`)
        .action((tokenId, tokenSymbol, options) => {
            mainProcessor(migrateCoinMetadata, options, [tokenId, tokenSymbol], processCommand);
        });

    program.addCommand(setFlowLimitsProgram);
    program.addCommand(addTrustedChainsProgram);
    program.addCommand(removeTrustedChainsProgram);
    program.addCommand(migrateCoinMetadataProgram);

    addOptionsToCommands(program, addBaseOptions, { offline: true });

    program.parse();
}
