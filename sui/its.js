const { Command } = require('commander');
const { TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, getChainConfig } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet, printWalletInfo, broadcastFromTxBuilder } = require('./utils');

async function setupTrustedAddress(keypair, client, contracts, args, options) {
    const [trustedChain, trustedAddress] = args;

    const { ITS: itsConfig } = contracts;

    const { OwnerCap, ITS } = itsConfig.objects;

    const txBuilder = new TxBuilder(client);

    const trustedAddressesObject = await txBuilder.moveCall({
        target: `${itsConfig.address}::trusted_addresses::new`,
        arguments: [[trustedChain], [trustedAddress]],
    });

    await txBuilder.moveCall({
        target: `${itsConfig.address}::its::set_trusted_addresses`,
        arguments: [ITS, OwnerCap, trustedAddressesObject],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, 'Setup Trusted Address');

    // Add trusted address to ITS config
    if (!contracts.ITS.trustedAddresses) contracts.ITS.trustedAddresses = {};
    if (!contracts.ITS.trustedAddresses[trustedChain]) contracts.ITS.trustedAddresses[trustedChain] = [];

    contracts.ITS.trustedAddresses[trustedChain].push(trustedAddress);
}

async function processCommand(command, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    await command(keypair, client, chain.contracts, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const suiConfig = getChainConfig(config, 'sui');
    await processor(command, suiConfig, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('ITS ').description('SUI ITS scripts');

    // This command is used to setup the trusted address on the ITS contract.
    // The trusted address is used to verify the message from the source chain.
    const setupTrustedAddressProgram = new Command()
        .name('setup-trusted-address')
        .description('Setup trusted address')
        .command('setup-trusted-address <trusted-chain> <trusted-address>')
        .action((trustedChain, trustedAddress, options) => {
            mainProcessor(setupTrustedAddress, options, [trustedChain, trustedAddress], processCommand);
        });

    program.addCommand(setupTrustedAddressProgram);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
