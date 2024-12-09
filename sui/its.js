const { Command } = require('commander');
const { TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, getChainConfig } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet, printWalletInfo, broadcastFromTxBuilder, saveGeneratedTx } = require('./utils');

const SPECIAL_CHAINS_TAGS = {
    ALL_EVM: 'all-evm', // All EVM chains that have ITS deployed
};

function parseTrustedChains(config, trustedChain) {
    if (trustedChain === SPECIAL_CHAINS_TAGS.ALL_EVM) {
        const evmChains = Object.keys(config.chains).filter(
            (chain) => config.chains[chain].chainType === 'evm' && config.chains[chain].contracts.InterchainTokenService,
        );
        return evmChains;
    }

    return trustedChain.split(',');
}

async function setupTrustedAddress(keypair, client, config, contracts, args, options) {
    const [trustedChain, trustedAddress] = args;

    const { ITS: itsConfig } = contracts;

    const { OwnerCap, ITS } = itsConfig.objects;

    const txBuilder = new TxBuilder(client);

    const trustedChains = parseTrustedChains(config, trustedChain);

    const trustedAddressesObject = await txBuilder.moveCall({
        target: `${itsConfig.address}::trusted_addresses::new`,
        arguments: [trustedChains, trustedChains.map(() => trustedAddress)],
    });

    await txBuilder.moveCall({
        target: `${itsConfig.address}::its::set_trusted_addresses`,
        arguments: [ITS, OwnerCap, trustedAddressesObject],
    });

    if (options.offline) {
        const tx = txBuilder.tx;
        const sender = options.sender || keypair.toSuiAddress();
        tx.setSender(sender);
        await saveGeneratedTx(tx, `Set trusted address for ${trustedChain} to ${trustedAddress}`, client, options);
    } else {
        await broadcastFromTxBuilder(txBuilder, keypair, 'Setup Trusted Address');
    }

    // Update ITS config
    for (const trustedChain of trustedChains) {
        // Add trusted address to ITS config
        if (!contracts.ITS.trustedAddresses) contracts.ITS.trustedAddresses = {};

        contracts.ITS.trustedAddresses[trustedChain] = trustedAddress;
    }
}

async function removeTrustedAddress(keypair, client, contracts, args, options) {
    const [trustedChain] = args;

    const trustedAddressesObject = contracts.ITS.trustedAddresses;

    if (!trustedAddressesObject) throw new Error('No trusted addresses found');

    const chainNames = trustedChain.split(',');

    if (chainNames.length === 0) throw new Error('No chain names provided');

    const txBuilder = new TxBuilder(client);

    for (const chainName of chainNames) {
        if (!trustedAddressesObject[chainName]) throw new Error(`No trusted addresses found for chain ${trustedChain}`);
    }

    await txBuilder.moveCall({
        target: `${contracts.ITS.address}::its::remove_trusted_addresses`,
        arguments: [contracts.ITS.objects.ITS, contracts.ITS.objects.OwnerCap, chainNames],
    });

    for (const chainName of chainNames) {
        delete contracts.ITS.trustedAddresses[chainName];
    }

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
    program.name('ITS').description('SUI ITS scripts');

    // This command is used to setup the trusted address on the ITS contract.
    // The trusted address is used to verify the message from the source chain.
    const setupTrustedAddressProgram = new Command()
        .name('setup-trusted-address')
        .command('setup-trusted-address <trusted-chain> <trusted-address>')
        .description(
            `Setup trusted address. The <trusted-chain> can be a list of chains separated by commas. It can also be a special tag to indicate a specific set of chains e.g. '${SPECIAL_CHAINS_TAGS.ALL_EVM}' to target all ITS-deployed EVM chains`,
        )
        .action((trustedChain, trustedAddress, options) => {
            mainProcessor(setupTrustedAddress, options, [trustedChain, trustedAddress], processCommand);
        });

    const removeTrustedAddressProgram = new Command()
        .name('remove-trusted-address')
        .description('Remove trusted address')
        .command('remove-trusted-address <trusted-chain>')
        .action((trustedChain, options) => {
            mainProcessor(removeTrustedAddress, options, [trustedChain], processCommand);
        });

    program.addCommand(setupTrustedAddressProgram);
    program.addCommand(removeTrustedAddressProgram);

    addOptionsToCommands(program, addBaseOptions, { offline: true });

    program.parse();
}
