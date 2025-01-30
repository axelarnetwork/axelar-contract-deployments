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

function getVaraiblesForPackage(chain, packageName) {
    if (packageName === 'AxelarGateway') {
        const contractConfig = chain.contracts.AxelarGateway;
        return {
            packageId: contractConfig.address,
            singletonId: contractConfig.objects.Gateway,
            versionedId: contractConfig.objects.Gatewayv0,
            ownerCapId: contractConfig.objects.OwnerCap,
            moduleName: 'gateway',
            defaultFunctions: {
                versions: [0, 0],
                functionNames: ['approve_messages', 'rotate_signers'],
            },
            contract: contractConfig,
        };
    } else if (packageName === 'InterchainTokenService') {
        const contractConfig = chain.contracts.InterchainTokenService;
        return {
            packageId: contractConfig.address,
            singletonId: contractConfig.objects.InterchainTokenService,
            versionedId: contractConfig.objects.InterchainTokenServicev0,
            ownerCapId: contractConfig.objects.OwnerCap,
            moduleName: 'interchain_token_service',
            defaultFunctions: {
                versions: [0, 0, 0, 0, 0, 0, 0, 0, 0],
                functionNames: ['register_coin', 'deploy_remote_interchain_token', 'send_interchain_transfer', 'receive_interchain_transfer', 'receive_interchain_transfer_with_data', 'receive_deploy_interchain_token', 'mint_as_distributor', 'mint_to_as_distributor', 'burn_as_distributor' ],
            },
            contract: contractConfig,
        };
    } else {
        throw new Error(`Unknown package ${packageName}.`);
    }

}

async function allowFunctions(keypair, client, packageId, moduleName, singletonId, ownerCapId, versions, functionNames) {
    if (versions.length !== functionNames.length) throw new Error('Versions and Function Names must have a matching length');

    const builder = new TxBuilder(client);

    for (const i in versions) {
        await builder.moveCall({
            target: `${packageId}::${moduleName}::allow_function`,
            arguments: [singletonId, ownerCapId, versions[i], functionNames[i]],
        });
    }

    await broadcastFromTxBuilder(builder, keypair, 'Allow Functions');
}

async function disallowFunctions(keypair, client, packageId, moduleName, singletonId, ownerCapId, versions, functionNames) {
    if (versions.length !== functionNames.length) throw new Error('Versions and Function Names must have a matching length');

    const builder = new TxBuilder(client);

    for (const i in versions) {
        await builder.moveCall({
            target: `${packageId}::${moduleName}::disallow_function`,
            arguments: [singletonId, ownerCapId, versions[i], functionNames[i]],
        });
    }

    await broadcastFromTxBuilder(builder, keypair, 'Disallow Functions');
}

async function pause(keypair, client, chain, args, options) {
    const [packageName] = args;
    const functions = options.functions;

    const { packageId, singletonId, versionedId, ownerCapId, moduleName, defaultFunctions, contract } = getVaraiblesForPackage(
        chain,
        packageName,
    );

    let versionsArg = [];
    let allowedFunctionsArg = [];

    if (functions === SPECIAL_PAUSE_FUNCTION_TAGS.ALL) {
        const allowedFunctionsArray = await getAllowedFunctions(client, versionedId);

        for (const version in allowedFunctionsArray) {
            let allowedFunctions = allowedFunctionsArray[version];

            // Do not dissalow `allow_function` because that locks the gateway forever.
            if (Number(version) === allowedFunctionsArray.length - 1) {
                allowedFunctions = allowedFunctions.filter((allowedFunction) => allowedFunction !== 'allow_function');
            }

            printInfo(`Functions that will be disallowed for version ${version}`, allowedFunctions);

            versionsArg = versionsArg.concat(new Array(allowedFunctions.length).fill(Number(version)));
            allowedFunctionsArg = allowedFunctionsArg.concat(allowedFunctions);
        }
    } else if (functions === SPECIAL_PAUSE_FUNCTION_TAGS.DEFAULT) {
        versionsArg = defaultFunctions.versions;
        allowedFunctionsArg = defaultFunctions.functionNames;
    } else {
        const unparsedArray = functions.split(',');

        if (unparsedArray.length % 2 !== 0) {
            throw new Error('Custom functions to pause must be an even length array, pairs of version-function name.');
        }

        for (let i = 0; i < unparsedArray.length / 2; i++) {
            versionsArg.push(Number(unparsedArray[2 * i]));
            allowedFunctionsArg.push(unparsedArray[2 * i + 1]);
        }
    }

    if (!contract.disallowedFunctions) {
        contract.disallowedFunctions = {
            versions: [],
            functionNames: [],
        };
    }

    contract.disallowedFunctions.versions = contract.disallowedFunctions.versions.concat(versionsArg);
    contract.disallowedFunctions.functionNames = contract.disallowedFunctions.functionNames.concat(allowedFunctionsArg);

    return await disallowFunctions(keypair, client, packageId, moduleName, singletonId, ownerCapId, versionsArg, allowedFunctionsArg);
}

async function unpause(keypair, client, chain, args, options) {
    const [ packageName ] = args;
    const functions = options.functions;
    const { packageId, singletonId, ownerCapId, moduleName, defaultFunctions, contract } = getVaraiblesForPackage(chain, packageName);

    let versionsArg = [];
    let allowedFunctionsArg = [];

    if (functions === SPECIAL_UNPAUSE_FUNCTION_TAGS.DISALLOWED) {
        versionsArg = contract.disallowedFunctions.versions.slice();
        allowedFunctionsArg = contract.disallowedFunctions.functionNames.slice();
    } else if (functions === SPECIAL_UNPAUSE_FUNCTION_TAGS.DEFAULT) {
        versionsArg = defaultFunctions.versions;
        allowedFunctionsArg = defaultFunctions.functionNames;
    } else {
        const unparsedArray = functions.split(',');

        if (unparsedArray.length % 2 !== 0) {
            throw new Error('Custom functions to pause must be an even length array, pairs of version-function name.');
        }

        for (let i = 0; i < unparsedArray.length / 2; i++) {
            versionsArg.push(Number(unparsedArray[2 * i]));
            allowedFunctionsArg.push(unparsedArray[2 * i + 1]);
        }
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

    return await allowFunctions(keypair, client, packageId, moduleName, singletonId, ownerCapId, versionsArg, allowedFunctionsArg);
}

async function processCommand(command, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    await command(keypair, client, chain, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
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
                'The functions to allow. Use use "default" for the default functions, "all" for all functions except the most recent "allow_function" and a comma separated list for custom pausing. The comma separated list has to be alternating version numbers and function names.',
            ).default('default'),
        )
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
                'The functions to allow. Use use "disallowed" for previously disallowed functions, "default" for the default functions and a comma separated list for custom pausing. The comma separated list has to be alternating version numbers and function names.',
            ).default('disallowed'),
        )
        .action((packageName, options) => {
            mainProcessor(unpause, options, [packageName], processCommand);
        });

    program.addCommand(pauseProgram);
    program.addCommand(unpauseProgram);

    addOptionsToCommands(program, addBaseOptions, { offline: true });

    program.parse();
}
