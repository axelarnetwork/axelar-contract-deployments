const { Command, Option } = require('commander');
const { TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, getChainConfig, printInfo } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    getWallet,
    printWalletInfo,
    broadcastFromTxBuilder,
    getAllowedFunctions,
} = require('./utils');

const SPECIAL_PAUSE_FUNCTION_TAGS = {
    ALL: 'all', // All EVM chains that have InterchainTokenService deployed
    DEFAULT: 'default',
};

const SPECIAL_UNPAUSE_FUNCTION_TAGS = {
    DISALLOWED: 'disallowed', // All EVM chains that have InterchainTokenService deployed
    DEFAULT: 'default',
};

const CONTRACT_INFO = {
    AxelarGateway: {
        singletonName: 'Gateway',
        moduleName: 'gateway',
        defaultFunctions: {
            versions: [0, 0],
            functionNames: ['approve_messages', 'rotate_signers'],
        },
    },
    InterchainTokenService: {
        singletonName: 'InterchainTokenService',
        moduleName: 'interchain_token_service',
        defaultFunctions: {
            versions: [0, 0, 0, 0, 0, 0, 0, 0],
            functionNames: [
                'deploy_remote_interchain_token',
                'send_interchain_transfer',
                'receive_interchain_transfer',
                'receive_interchain_transfer_with_data',
                'receive_deploy_interchain_token',
                'mint_as_distributor',
                'mint_to_as_distributor',
                'burn_as_distributor',
            ],
        },
    },
};

function getVariablesForPackage(chain, packageName) {
    const contractConfig = chain.contracts[packageName];
    const info = CONTRACT_INFO[packageName];
    const defaultFunctions = info.defaultFunctions;
    const version = Math.max(...Object.keys(contractConfig.versions).map((version) => Number(version)));
    const defaultFunctions = { ...info.defaultFunctions, versions: info.defaultFunctions.versions.map(() => version) };
    return {
        packageId: contractConfig.address,
        singletonId: contractConfig.objects[info.singletonName],
        versionedId: contractConfig.objects[info.singletonName + 'v0'],
        ownerCapId: contractConfig.objects.OwnerCap,
        moduleName: info.moduleName,
        defaultFunctions: info.defaultFunctions,
        contract: contractConfig,
    };
}

async function allowFunctions(keypair, client, packageId, moduleName, singletonId, ownerCapId, versions, functionNames, options) {
    if (versions.length !== functionNames.length) throw new Error('Versions and Function Names must have a matching length');

    const builder = new TxBuilder(client);

    for (let i = 0; i < versions.length; i++) {
        await builder.moveCall({
            target: `${packageId}::${moduleName}::allow_function`,
            arguments: [singletonId, ownerCapId, versions[i], functionNames[i]],
        });
    }

    await broadcastFromTxBuilder(builder, keypair, 'Allow Functions', options);
}

async function disallowFunctions(keypair, client, packageId, moduleName, singletonId, ownerCapId, versions, functionNames, options) {
    if (versions.length !== functionNames.length) throw new Error('Versions and Function Names must have a matching length');

    const builder = new TxBuilder(client);

    for (let i = 0; i < versions.length; i++) {
        await builder.moveCall({
            target: `${packageId}::${moduleName}::disallow_function`,
            arguments: [singletonId, ownerCapId, versions[i], functionNames[i]],
        });
    }

    await broadcastFromTxBuilder(builder, keypair, 'Disallow Functions', options);
}

async function pause(keypair, client, chain, args, options) {
    const [packageName] = args;
    const functions = options.functions;

    const { packageId, singletonId, versionedId, ownerCapId, moduleName, defaultFunctions, contract } = getVariablesForPackage(
        chain,
        packageName,
    );

    let versionsArg = [];
    let allowedFunctionsArg = [];

    if (functions === SPECIAL_PAUSE_FUNCTION_TAGS.ALL) {
        const allowedFunctionsArray = await getAllowedFunctions(client, versionedId);

        for (let version = 0; version < allowedFunctionsArray.length; version++) {
            if (options.version !== 'all' && options.version !== String(version)) {
                continue;
            }

            let allowedFunctions = allowedFunctionsArray[version];

            // Do not disable `allow_function` because that locks the contract forever.
            allowedFunctions = allowedFunctions.filter((allowedFunction) => {
                return allowedFunction !== 'allow_function' && allowedFunction !== 'disallow_function';
            });

            printInfo(`Functions that will be disallowed for version ${version}`, allowedFunctions);

            versionsArg = versionsArg.concat(new Array(allowedFunctions.length).fill(Number(version)));
            allowedFunctionsArg = allowedFunctionsArg.concat(allowedFunctions);
        }
    } else if (functions === SPECIAL_PAUSE_FUNCTION_TAGS.DEFAULT) {
        versionsArg = defaultFunctions.versions;
        allowedFunctionsArg = defaultFunctions.functionNames;
    } else if (options.version !== 'all') {
        allowedFunctionsArg = functions.split(',');
        versionsArg = allowedFunctionsArg.map(() => Number(options.version));
    } else {
        throw new Error('Need to specify a version if providing specific functions.');
    }

    if (!contract.disallowedFunctions) {
        contract.disallowedFunctions = {
            versions: [],
            functionNames: [],
        };
    }

    contract.disallowedFunctions.versions = contract.disallowedFunctions.versions.concat(versionsArg);
    contract.disallowedFunctions.functionNames = contract.disallowedFunctions.functionNames.concat(allowedFunctionsArg);

    return disallowFunctions(keypair, client, packageId, moduleName, singletonId, ownerCapId, versionsArg, allowedFunctionsArg, options);
}

async function unpause(keypair, client, chain, args, options) {
    const [packageName] = args;
    const functions = options.functions;
    const { packageId, singletonId, ownerCapId, moduleName, defaultFunctions, contract } = getVariablesForPackage(chain, packageName);

    let versionsArg = [];
    let allowedFunctionsArg = [];

    if (functions === SPECIAL_UNPAUSE_FUNCTION_TAGS.DISALLOWED) {
        versionsArg = contract.disallowedFunctions.versions.slice();
        allowedFunctionsArg = contract.disallowedFunctions.functionNames.slice();
    } else if (functions === SPECIAL_UNPAUSE_FUNCTION_TAGS.DEFAULT) {
        versionsArg = defaultFunctions.versions;
        allowedFunctionsArg = defaultFunctions.functionNames;
    } else if (options.version !== 'all') {
        allowedFunctionsArg = functions.split(',');
        versionsArg = allowedFunctionsArg.map(() => Number(options.version));
    } else {
        throw new Error('Need to specify a version if providing specific functions.');
    }

    if (contract.disallowedFunctions) {
        for (let i = contract.disallowedFunctions.versions.length - 1; i >= 0; i--) {
            const version = contract.disallowedFunctions.versions[i];
            const functionName = contract.disallowedFunctions.functionNames[i];

            for (let j = 0; j < versionsArg.length; j++) {
                if (version === versionsArg[j] && functionName === allowedFunctionsArg[j]) {
                    contract.disallowedFunctions.versions.splice(i, 1);
                    contract.disallowedFunctions.functionNames.splice(i, 1);
                    break;
                }
            }
        }
    }

    return allowFunctions(keypair, client, packageId, moduleName, singletonId, ownerCapId, versionsArg, allowedFunctionsArg, options);
}

async function processCommand(command, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    await command(keypair, client, chain, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(command, chain, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('Pause').description('SUI Pause scripts');

    const pauseProgram = new Command()
        .name('pause')
        .description('Pause')
        .command('pause <package>')
        .addOption(
            new Option(
                '--functions <functions>',
                'The functions to allow. Use "default" for the default functions, "all" for all functions except the most recent "allow_function" and a comma separated list for custom pausing.',
            ).default('default'),
        )
        .addOption(new Option('--version, <version>', 'The version to pause. Use all to pause all versions').default('all'))
        .action((packageName, options) => {
            mainProcessor(pause, options, [packageName], processCommand);
        });

    const unpauseProgram = new Command()
        .name('unpause')
        .description('Unpause')
        .command('unpause <package>')
        .addOption(
            new Option(
                '--functions, <functions>',
                'The functions to pause. Use "disallowed" for previously disallowed functions, "default" for the default functions and a comma separated list for custom pausing.',
            ).default('disallowed'),
        )
        .addOption(new Option('--version, <version>', 'The version to pause. Use all to pause all versions').default('all'))
        .action((packageName, options) => {
            mainProcessor(unpause, options, [packageName], processCommand);
        });

    program.addCommand(pauseProgram);
    program.addCommand(unpauseProgram);

    addOptionsToCommands(program, addBaseOptions, { offline: true });

    program.parse();
}
