const { Command } = require('commander');
const { TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, getChainConfig } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet, printWalletInfo, broadcastFromTxBuilder, saveGeneratedTx } = require('./utils');

const SPECIAL_CHAINS_TAGS = {
    ALL_EVM: 'all-evm', // All EVM chains that have InterchainTokenService deployed
};

function parseTrustedChains(config, trustedChain) {
    if (trustedChain === SPECIAL_CHAINS_TAGS.ALL_EVM) {
        const evmChains = Object.keys(config.chains).filter(
            (chain) => config.chains[chain].contracts?.InterchainTokenService?.address,
        );
        return evmChains;
    }

    return trustedChain.split(',');
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
        await broadcastFromTxBuilder(txBuilder, keypair, 'Setup Trusted Address');
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

    await broadcastFromTxBuilder(txBuilder, keypair, 'Remove Trusted Address');
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
        .command('add-trusted-chains <trusted-chain>')
        .description(
            `Add trusted chain. The <trusted-chain> can be a list of chains separated by commas. It can also be a special tag to indicate a specific set of chains e.g. '${SPECIAL_CHAINS_TAGS.ALL_EVM}' to target all InterchainTokenService-deployed EVM chains`,
        )
        .action((trustedChain, options) => {
            mainProcessor(addTrustedChains, options, [trustedChain], processCommand);
        });

    const removeTrustedChainsProgram = new Command()
        .name('remove-trusted-address')
        .description('Remove trusted address')
        .command('remove-trusted-address <trusted-chain>')
        .action((trustedChain, options) => {
            mainProcessor(removeTrustedChain, options, [trustedChain], processCommand);
        });

    program.addCommand(setupTrustedChainsProgram);
    program.addCommand(removeTrustedChainsProgram);

    addOptionsToCommands(program, addBaseOptions, { offline: true });

    program.parse();
}
