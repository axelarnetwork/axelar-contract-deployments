const { Command } = require('commander');
const { TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, getChainConfig } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet, printWalletInfo, broadcastFromTxBuilder } = require('./utils');

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

    await broadcastFromTxBuilder(txBuilder, keypair, 'Setup Trusted Addresses');

    // Update ITS config
    for (const trustedChain of trustedChains) {
        // Add trusted address to ITS config
        if (!contracts.ITS.trustedAddresses) contracts.ITS.trustedAddresses = {};

        contracts.ITS.trustedAddresses[trustedChain] = trustedAddress;
    }
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
    program.name('ITS ').description('SUI ITS scripts');

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

    program.addCommand(setupTrustedAddressProgram);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
