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
            (chain) => config.chains[chain].chainType === 'evm' && config.chains[chain].contracts.InterchainTokenService,
        );
        return evmChains;
    }

    return trustedChain.split(',');
}

async function setupTrustedAddress(keypair, client, config, contracts, args, options) {
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

    // Update InterchainTokenService config
    for (const trustedChain of trustedChains) {
        // Add trusted address to InterchainTokenService config
        if (!contracts.InterchainTokenService.trustedAddresses) contracts.InterchainTokenService.trustedAddresses = [];

        contracts.InterchainTokenService.trustedAddresses.push(trustedChain);
    }
}

async function removeTrustedAddress(keypair, client, contracts, args, options) {
    const [trustedChain] = args;

    const trustedAddressesObject = contracts.InterchainTokenService.trustedAddresses;

    if (!trustedAddressesObject) throw new Error('No trusted addresses found');

    const chainNames = trustedChain.split(',');

    if (chainNames.length === 0) throw new Error('No chain names provided');

    const txBuilder = new TxBuilder(client);

    for (const chainName of chainNames) {
        if (!trustedAddressesObject[chainName]) throw new Error(`No trusted addresses found for chain ${trustedChain}`);
    }

    await txBuilder.moveCall({
        target: `${contracts.InterchainTokenService.address}::interchain_token_service::remove_trusted_chains`,
        arguments: [
            contracts.InterchainTokenService.objects.InterchainTokenService,
            contracts.InterchainTokenService.objects.OwnerCap,
            chainNames,
        ],
    });

    for (const chainName of chainNames) {
        delete contracts.InterchainTokenService.trustedAddresses[chainName];
    }

    await broadcastFromTxBuilder(txBuilder, keypair, 'Remove Trusted Address');
}

async function allowFunctions(keypair, client, config, contractConfig, args, options) {
    const contracts = contractConfig.ITS;
    console.log(contracts);
    const packageId = contracts.address;

    const [versionsArg, functionNamesArg] = args;

    const versions = versionsArg.split(',');
    const functionNames = functionNamesArg.split(',');

    if (versions.length !== functionNames.length) throw new Error('Versions and Function Names must have a matching length');

    const builder = new TxBuilder(client);

    for (const i in versions) {
        await builder.moveCall({
            target: `${packageId}::interchain_token_service::allow_function`,
            arguments: [contracts.objects.InterchainTokenService, contracts.objects.OwnerCap, versions[i], functionNames[i]],
        });
    }

    return {
        tx,
        message: 'Allow Functions',
    };
}

async function disallowFunctions(keypair, client, config, contractConfig, args, options) {
    const packageId = contractConfig.address;

    const [versionsArg, functionNamesArg] = args;

    const versions = versionsArg.split(',');
    const functionNames = functionNamesArg.split(',');

    if (versions.length !== functionNames.length) throw new Error('Versions and Function Names must have a matching length');

    const tx = new Transaction();

    for (const i in versions) {
        tx.moveCall({
            target: `${packageId}::gateway::disallow_function`,
            arguments: [
                tx.object(contractConfig.objects.Gateway),
                tx.object(contractConfig.objects.OwnerCap),
                tx.pure.u64(versions[i]),
                tx.pure.string(functionNames[i]),
            ],
        });
    }

    return {
        tx,
        message: 'Disallow Functions',
    };
}

async function pause(keypair, client, config, contracts, args, options) {
    const response = await client.getObject({
        id: contracts.objects.Gatewayv0,
        options: {
            showContent: true,
            showBcs: true,
        },
    });
    let allowedFunctionsArray = response.data.content.fields.value.fields.version_control.fields.allowed_functions;
    allowedFunctionsArray = allowedFunctionsArray.map((allowedFunctions) => allowedFunctions.fields.contents);

    const versionsArg = [];
    const allowedFunctionsArg = [];

    for (const version in allowedFunctionsArray) {
        const allowedFunctions = allowedFunctionsArray[version];

        // Do not dissalow `allow_function` because that locks the gateway forever.
        if (version == allowedFunctionsArray.length - 1) {
            const index = allowedFunctions.indexOf('allow_function');

            if (index > -1) {
                // only splice array when item is found
                allowedFunctions.splice(index, 1); // 2nd parameter means remove one item only
            }
        }

        printInfo(`Functions that will be disallowed for version ${version}`, allowedFunctions);

        versionsArg.push(new Array(allowedFunctions.length).fill(version).join());
        allowedFunctionsArg.push(allowedFunctions.join());
    }

    // Write the
    writeJSON(
        {
            versions: versionsArg,
            disallowedFunctions: allowedFunctionsArg,
        },
        `${__dirname}/../axelar-chains-config/info/sui-allowed-functions-${options.env}.json`,
    );

    return disallowFunctions(keypair, client, config, contracts, [versionsArg.join(), allowedFunctionsArg.join()], options);
}

async function unpause(keypair, client, config, contracts, args, options) {
    const dissalowedFunctions = readJSON(`${__dirname}/../axelar-chains-config/info/sui-allowed-functions-${options.env}.json`);

    return allowFunctions(
        keypair,
        client,
        config,
        contracts,
        [dissalowedFunctions.versions.join(), dissalowedFunctions.disallowedFunctions.join()],
        options,
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

    // This command is used to setup the trusted address on the InterchainTokenService contract.
    // The trusted address is used to verify the message from the source chain.
    const setupTrustedAddressProgram = new Command()
        .name('setup-trusted-address')
        .command('setup-trusted-address <trusted-chain>')
        .description(
            `Add trusted chain. The <trusted-chain> can be a list of chains separated by commas. It can also be a special tag to indicate a specific set of chains e.g. '${SPECIAL_CHAINS_TAGS.ALL_EVM}' to target all InterchainTokenService-deployed EVM chains`,
        )
        .action((trustedChain, options) => {
            mainProcessor(setupTrustedAddress, options, [trustedChain], processCommand);
        });

    const removeTrustedAddressProgram = new Command()
        .name('remove-trusted-address')
        .description('Remove trusted address')
        .command('remove-trusted-address <trusted-chain>')
        .action((trustedChain, options) => {
            mainProcessor(removeTrustedAddress, options, [trustedChain], processCommand);
        });

    const allowFunctionsProgram = new Command()
        .name('allow-functions')
        .description('Allow functions')
        .command('allow-functions <versions> <functions>')
        .action((versions, functions, options) => {
            mainProcessor(allowFunctions, options, [versions, functions], processCommand);
        });

    program.addCommand(setupTrustedAddressProgram);
    program.addCommand(removeTrustedAddressProgram);
    program.addCommand(allowFunctionsProgram);

    addOptionsToCommands(program, addBaseOptions, { offline: true });

    program.parse();
}
