const { Command } = require('commander');
const { TxBuilder, STD_PACKAGE_ID } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, getChainConfig, parseTrustedChains } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    getWallet,
    printWalletInfo,
    broadcastFromTxBuilder,
    saveGeneratedTx,
    suiClockAddress,
    suiCoinId,
} = require('./utils');
const { bcs } = require('@mysten/sui/bcs');
const {
    utils: { arrayify, parseUnits },
} = require('hardhat').ethers;

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

async function interchainTransfer(keypair, client, config, contracts, args, options) {
    const { InterchainTokenService: itsConfig } = contracts;

    const { InterchainTokenService } = itsConfig.objects;

    const { coinPackageId, coinPackageName, coinModName, coinObjectId, tokenId, destinationChain, destinationAddress, amount } = options;

    const walletAddress = keypair.toSuiAddress();

    const txBuilder = new TxBuilder(client);
    const tx = txBuilder.tx;

    const coinType = `${coinPackageId}::${coinPackageName}::${coinModName}`;


    const tokenIdObj = await txBuilder.moveCall({
        target: `${contracts.InterchainTokenService.address}::token_id::from_u256`,
        arguments: [tokenId],
    });

    const gatewayChannelId = await txBuilder.moveCall({
        target: `${contracts.AxelarGateway.address}::channel::new`,
        arguments: [],
    });

    // Split coins to set exact amount of coins to send.
    const [coinsToSend] = tx.splitCoins(coinObjectId, [amount]);

    const prepareInterchainTransferTicket = await txBuilder.moveCall({
        target: `${contracts.InterchainTokenService.address}::interchain_token_service::prepare_interchain_transfer`,
        typeArguments: [coinType],
        arguments: [tokenIdObj, coinsToSend, destinationChain, destinationAddress, '0x', gatewayChannelId],
    });

    const interchainTransferTicket = await txBuilder.moveCall({
        target: `${contracts.InterchainTokenService.address}::interchain_token_service::send_interchain_transfer`,
        typeArguments: [coinType],
        arguments: [InterchainTokenService, prepareInterchainTransferTicket, suiClockAddress],
    });

    // Specify one unit of gas to be paid to gas service.
    const unitAmountGas = parseUnits('1', 9).toBigInt();

    const [gas] = tx.splitCoins(tx.gas, [unitAmountGas]);

    await txBuilder.moveCall({
        target: `${contracts.GasService.address}::gas_service::pay_gas`,
        typeArguments: [suiCoinId],
        arguments: [contracts.GasService.objects.GasService, interchainTransferTicket, gas, walletAddress, '0x'],
    });

    await txBuilder.moveCall({
        target: `${contracts.AxelarGateway.address}::gateway::send_message`,
        arguments: [contracts.AxelarGateway.objects.Gateway, interchainTransferTicket],
    });

    await txBuilder.moveCall({
        target: `${contracts.AxelarGateway.address}::channel::destroy`,
        arguments: [gatewayChannelId],
    });

    if (options.offline) {
        const tx = txBuilder.tx;
        const sender = options.sender || keypair.toSuiAddress();
        tx.setSender(sender);
        await saveGeneratedTx(tx, `Interchain transfer for ${tokenId}`, client, options);
    } else {
        await broadcastFromTxBuilder(txBuilder, keypair, 'Interchain Transfer', options);
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

    const interchainTransferProgram = new Command()
        .name('interchain-transfer')
        .command('interchain-transfer')
        .description('Send interchain transfer from sui to a chain where token is linked')
        .requiredOption('--coin-package-id <coinPackageId>', 'The coin package ID')
        .requiredOption('--coin-package-name <coinPackageName>', 'The coin package name')
        .requiredOption('--coin-mod-name <coinModName>', 'The coin module name')
        .requiredOption('--coin-object-id <coinObjectId>', 'The coin object ID')
        .requiredOption('--token-id <tokenId>', 'The token ID')
        .requiredOption('--destination-chain <destinationChain>', 'The destination chain')
        .requiredOption('--destination-address <destinationAddress>', 'The destination address')
        .requiredOption('--amount <amount>', 'The amount to transfer')
        .action((options) => {
            mainProcessor(interchainTransfer, options, [], processCommand);
        });

    program.addCommand(setFlowLimitsProgram);
    program.addCommand(addTrustedChainsProgram);
    program.addCommand(removeTrustedChainsProgram);
    program.addCommand(interchainTransferProgram);

    addOptionsToCommands(program, addBaseOptions, { offline: true });

    program.parse();
}
