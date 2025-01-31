const { Command } = require('commander');
const { TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, getChainConfig, printInfo, writeJSON } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet, printWalletInfo, broadcastFromTxBuilder, saveGeneratedTx } = require('./utils');
const { readJSON } = require(`${__dirname}/../axelar-chains-config`);

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

async function setupTrustedChain(keypair, client, config, contracts, args, options) {
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

async function allowFunctions(keypair, client, config, contractConfig, args, options) {
    const contracts = contractConfig.InterchainTokenService;
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

    await broadcastFromTxBuilder(builder, keypair, 'Allow Functions');
}

async function disallowFunctions(keypair, client, config, contractConfig, args, options) {
    const contracts = contractConfig.InterchainTokenService;
    const packageId = contracts.address;

    const [versionsArg, functionNamesArg] = args;

    const versions = versionsArg.split(',');
    const functionNames = functionNamesArg.split(',');

    if (versions.length !== functionNames.length) throw new Error('Versions and Function Names must have a matching length');

    const builder = new TxBuilder(client);

    for (const i in versions) {
        await builder.moveCall({
            target: `${packageId}::interchain_token_service::disallow_function`,
            arguments: [contracts.objects.InterchainTokenService, contracts.objects.OwnerCap, versions[i], functionNames[i]],
        });
    }

    await broadcastFromTxBuilder(builder, keypair, 'Disallow Functions');
}

async function pause(keypair, client, config, contracts, args, options) {
    const response = await client.getObject({
        id: contracts.InterchainTokenService.objects.InterchainTokenServicev0,
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
        if (Number(version) === allowedFunctionsArray.length - 1) {
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
        `${__dirname}/../axelar-chains-config/info/sui-its-allowed-functions-${options.env}.json`,
    );

    return await disallowFunctions(keypair, client, config, contracts, [versionsArg.join(), allowedFunctionsArg.join()], options);
}

async function unpause(keypair, client, config, contracts, args, options) {
    const dissalowedFunctions = readJSON(`${__dirname}/../axelar-chains-config/info/sui-its-allowed-functions-${options.env}.json`);

    return await allowFunctions(
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
    const setupTrustedChainsProgram = new Command()
        .name('add-trusted-chains')
        .command('add-trusted-chains <trusted-chain>')
        .description(
            `Add trusted chain. The <trusted-chain> can be a list of chains separated by commas. It can also be a special tag to indicate a specific set of chains e.g. '${SPECIAL_CHAINS_TAGS.ALL_EVM}' to target all InterchainTokenService-deployed EVM chains`,
        )
        .action((trustedChain, options) => {
            mainProcessor(setupTrustedChain, options, [trustedChain], processCommand);
        });

    const removeTrustedChainsProgram = new Command()
        .name('remove-trusted-address')
        .description('Remove trusted address')
        .command('remove-trusted-address <trusted-chain>')
        .action((trustedChain, options) => {
            mainProcessor(removeTrustedChain, options, [trustedChain], processCommand);
        });

    const allowFunctionsProgram = new Command()
        .name('allow-functions')
        .description('Allow functions')
        .command('allow-functions <versions> <functions>')
        .action((versions, functions, options) => {
            mainProcessor(allowFunctions, options, [versions, functions], processCommand);
        });

    const pauseProgram = new Command()
        .name('pause')
        .description('Pause InterchainTokenService')
        .command('pause')
        .action((options) => {
            mainProcessor(pause, options, [], processCommand);
        });

    const unpauseProgram = new Command()
        .name('unpause')
        .description('Unpause InterchainTokenService')
        .command('unpause')
        .action((options) => {
            mainProcessor(unpause, options, [], processCommand);
        });

    program.addCommand(setupTrustedChainsProgram);
    program.addCommand(removeTrustedChainsProgram);
    program.addCommand(allowFunctionsProgram);
    program.addCommand(pauseProgram);
    program.addCommand(unpauseProgram);

    addOptionsToCommands(program, addBaseOptions, { offline: true });

    program.parse();
}
