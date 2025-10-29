'use strict';

const { Command, Option } = require('commander');
const { addBaseOptions, addOptionsToCommands, encodeITSDestination, loadConfig, printInfo, parseTrustedChains } = require('../common');

const { processCommand: evmProcessCommand } = require('../evm/its');
const { addTrustedChains: addTrustedChainsSui } = require('../sui/its');
const { addTrustedChains: addTrustedChainsStellar } = require('../stellar/its');

const { getWallet: getSuiWallet, printWalletInfo: printSuiWalletInfo } = require('../sui/utils');
const { getWallet: getStellarWallet } = require('../stellar/utils');
const { Contract: StellarContract } = require('@stellar/stellar-sdk');

const ALL_CHAINS = 'all';

async function encodeRecipient(config, args, _) {
    const [destinationChain, destinationAddress] = args;

    const itsDestinationAddress = encodeITSDestination(config.chains, destinationChain, destinationAddress);

    printInfo('Human-readable destination address', destinationAddress);
    printInfo('Encoded ITS destination address', itsDestinationAddress);
}

async function callEvmSetTrustedChains(config, evmPrivateKey, env) {
    const allEvmChains = Object.values(config.chains)
        .filter((c) => c.contracts?.InterchainTokenService?.address)
        .filter((c) => c.chainType === 'evm');

    const trustedChains = parseTrustedChains(config.chains, [ALL_CHAINS]);

    for (const chain of allEvmChains) {
        printInfo(`\n--- Setting trusted chains on ${chain.name} (${chain.axelarId}) ---`);

        const options = {
            privateKey: evmPrivateKey,
            args: trustedChains,
            env: env,
        };

        await evmProcessCommand(config.axelar, chain, config.chains, 'set-trusted-chains', options);
    }
}

async function callSuiAddTrustedChains(config, options) {
    const suiChain = Object.values(config.chains).find(
        (c) => c.contracts?.InterchainTokenService?.address && c.networkType?.includes('sui'),
    );

    if (!suiChain) {
        return;
    }

    printInfo(`\n--- Setting trusted chains on ${suiChain.name} (${suiChain.axelarId}) ---`);

    const [keypair, client] = getSuiWallet(suiChain, {
        privateKey: options.suiPrivateKey,
        signatureScheme: options.suiSignatureScheme,
        privateKeyType: options.suiPrivateKeyType,
    });

    await printSuiWalletInfo(keypair, client, suiChain, {});

    await addTrustedChainsSui(keypair, client, config, suiChain.contracts, [ALL_CHAINS], options);
}

async function callStellarAddTrustedChains(config, options) {
    const stellarChain = Object.values(config.chains).find(
        (c) => c.contracts?.InterchainTokenService?.address && c.horizonRpc !== undefined,
    );

    if (!stellarChain) {
        return;
    }

    printInfo(`\n--- Setting trusted chains on ${stellarChain.name} (${stellarChain.axelarId}) ---`);

    const wallet = await getStellarWallet(stellarChain, { privateKey: options.stellarPrivateKey });

    const contract = new StellarContract(stellarChain.contracts.InterchainTokenService.address);

    await addTrustedChainsStellar(wallet, config, stellarChain, contract, [ALL_CHAINS], options);
}

async function setTrustedChainsAll(config, args, options) {
    const { evmPrivateKey, suiPrivateKey, stellarPrivateKey } = options;

    printInfo('Setting trusted chains on all EVM chains...\n');
    await callEvmSetTrustedChains(config, evmPrivateKey, options.env);

    printInfo('Setting trusted chains for Sui...');
    await callSuiAddTrustedChains(config, options);

    printInfo('Setting trusted chains for Stellar...');
    await callStellarAddTrustedChains(config, options);
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);

    await processor(config, args, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('its').description('Interchain Token Service common operations.');

    program
        .command('encode-recipient <destination-chain> <destination-address>')
        .description('Encode ITS recipient based on destination chain in config')
        .action((destinationChain, destinationAddress, options) => {
            mainProcessor(encodeRecipient, [destinationChain, destinationAddress], options);
        });

    program
        .command('set-trusted-chains-all')
        .description('Set trusted chains for all chains')
        .addOption(
            new Option('--evmPrivateKey <evmPrivateKey>', 'Private key for EVM scripts').env('PRIVATE_KEY_EVM').makeOptionMandatory(true),
        )
        .addOption(
            new Option('--suiPrivateKey <suiPrivateKey>', 'Private key for Sui scripts').env('PRIVATE_KEY_SUI').makeOptionMandatory(true),
        )
        .addOption(
            new Option('--stellarPrivateKey <stellarPrivateKey>', 'Private key for Stellar scripts')
                .env('PRIVATE_KEY_STELLAR')
                .makeOptionMandatory(true),
        )
        .addOption(new Option('-y, --yes', 'Skip confirmation prompts').default(false))
        .addOption(new Option('--suiSignatureScheme <suiSignatureScheme>', 'Signature scheme for Sui').default('secp256k1'))
        .addOption(new Option('--suiPrivateKeyType <suiPrivateKeyType>', 'Private key type for Sui').default('mnemonic'))
        .action((options) => {
            mainProcessor(setTrustedChainsAll, [], options);
        });

    addOptionsToCommands(program, addBaseOptions, { ignoreChainNames: true, ignorePrivateKey: true });

    program.parse();
}
