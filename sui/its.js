const { Command } = require('commander');
const { TxBuilder, STD_PACKAGE_ID } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, getChainConfig, getITSChains, isValidChain } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet, printWalletInfo, broadcastFromTxBuilder, saveGeneratedTx } = require('./utils');
const { bcs } = require('@mysten/sui/bcs');

function parseTrustedChains(config, trustedChains) {
    if (trustedChains[0] === 'all' && trustedChains.length === 1) {
        return getITSChains(config);
    }

    trustedChains.forEach((trustedChain) => {
        isValidChain(config, trustedChain);
    });
    return trustedChains;
}

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
        await broadcastFromTxBuilder(txBuilder, keypair, 'Setup Trusted Address', options);
    }
}

async function addTrustedChains(keypair, client, config, contracts, args, options) {
    const [trustedChain] = args;

    const { InterchainTokenService: itsConfig } = contracts;

    const { OwnerCap, InterchainTokenService } = itsConfig.objects;

    const txBuilder = new TxBuilder(client);

    const trustedChains = parseTrustedChains(config, trustedChain);

    await txBuilder.moveCall({
        target: `${itsConfig.address}::interchain_token_service::add_trusted_chains`,
        arguments: [InterchainTokenService, OwnerCap, trustedChains],
    });

    if (options.offline) {
        const tx = txBuilder.tx;
        const sender = options.sender || keypair.toSuiAddress();
        tx.setSender(sender);
        await saveGeneratedTx(tx, `Added trusted chain ${trustedChain}`, client, options);
    } else {
        await broadcastFromTxBuilder(txBuilder, keypair, 'Setup Trusted Address', options);
    }
}

async function removeTrustedChain(keypair, client, contracts, args, options) {
    const [trustedChain] = args;

    const chainNames = trustedChain.split(',');

    if (chainNames.length === 0) throw new Error('No chain names provided');

    const txBuilder = new TxBuilder(client);

    await txBuilder.moveCall({
        target: `${contracts.InterchainTokenService.address}::interchain_token_service::remove_trusted_chains`,
        arguments: [
            contracts.InterchainTokenService.objects.InterchainTokenService,
            contracts.InterchainTokenService.objects.OwnerCap,
            chainNames,
        ],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, 'Remove Trusted Address', options);
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

    // This command is used to setup the trusted address on the InterchainTokenService contract.
    // The trusted address is used to verify the message from the source chain.
    const addTrustedChainsProgram = new Command()
        .name('add-trusted-chains')
        .command('add-trusted-chains <trusted-chains...>')
        .description(
            `Add trusted chains. The <trusted-chains> can be a list of chains separated by commas. It can also be a special tag to indicate a specific set of chains e.g. 'all' to target all InterchainTokenService-deployed chains`,
        )
        .action((trustedChains, options) => {
            mainProcessor(addTrustedChains, options, [trustedChains], processCommand);
        });

    const removeTrustedChainsProgram = new Command()
        .name('remove-trusted-address')
        .description('Remove trusted address')
        .command('remove-trusted-address <trusted-chains...>')
        .action((trustedChains, options) => {
            mainProcessor(removeTrustedChain, options, [trustedChains], processCommand);
        });

    const setFlowLimitsProgram = new Command()
        .name('set-flow-limits')
        .command('set-flow-limits <token-ids> <flow-limits>')
        .description(`Set flow limits for multiple tokens. <token-ids> and <flow-limits> can both be comma separated lists`)
        .action((tokenIds, flowLimits, options) => {
            mainProcessor(setFlowLimits, options, [tokenIds, flowLimits], processCommand);
        });

    program.addCommand(setFlowLimitsProgram);
    program.addCommand(addTrustedChainsProgram);
    program.addCommand(removeTrustedChainsProgram);

    addOptionsToCommands(program, addBaseOptions, { offline: true });

    program.parse();
}
