const fs = require('fs');
const { Command, Option } = require('commander');
const { copyMovePackage, getLocalDependencies, updateMoveToml, TxBuilder, bcsStructs } = require('@axelar-network/axelar-cgp-sui');
const { bcs } = require('@mysten/sui/bcs');
const { Transaction } = require('@mysten/sui/transactions');
const { saveConfig, printInfo, printWarn, validateParameters, getDomainSeparator, loadConfig, getChainConfig } = require('../common');
const {
    addBaseOptions,
    addOptionsToCommands,
    getWallet,
    printWalletInfo,
    broadcast,
    upgradePackage,
    getSigners,
    deployPackage,
    getObjectIdsByObjectTypes,
    suiPackageAddress,
    suiClockAddress,
    readMovePackageName,
    getSingletonChannelId,
    getItsChannelId,
    checkSuiVersionMatch,
    moveDir,
    getStructs,
    restrictUpgradePolicy,
    broadcastRestrictedUpgradePolicy,
    broadcastFromTxBuilder,
} = require('./utils');
const GatewayCli = require('./gateway');

/**
 * Move Package Directories
 *
 * This array contains the names of Move package directories located in:
 * `node_modules/@axelar-network/axelar-cgp-sui/move`
 *
 * Each string in this array corresponds to a folder name within that path.
 *
 * To deploy a new package:
 * 1. Add the new package's folder name to this array
 * 2. Ensure the corresponding folder exists in the specified path
 *
 */
const PACKAGE_DIRS = [
    'version_control',
    'utils',
    'gas_service',
    'example',
    'relayer_discovery',
    'axelar_gateway',
    'operators',
    'abi',
    'governance',
    'interchain_token_service',
    'interchain_token',
];

/**
 * Package Mapping Object for Command Options and Post-Deployment Functions
 */
const PACKAGE_CONFIGS = {
    cmdOptions: {
        AxelarGateway: () => GATEWAY_CMD_OPTIONS,
    },
    postDeployFunctions: {
        AxelarGateway: postDeployAxelarGateway,
        RelayerDiscovery: postDeployRelayerDiscovery,
        GasService: postDeployGasService,
        Example: postDeployExample,
        Operators: postDeployOperators,
        InterchainTokenService: postDeployIts,
        Utils: postDeployUtils,
        Abi: postDeployAbi,
        VersionControl: postDeployVersionControl,
    },
};

/**
 * Supported Move Packages
 *
 * Maps each directory in PACKAGE_DIRS to an object containing:
 * - packageName: Read from 'Move.toml' in the directory
 * - packageDir: The directory name
 *
 */
const supportedPackages = PACKAGE_DIRS.map((dir) => ({
    packageName: readMovePackageName(dir),
    packageDir: dir,
}));

/**
 * Post-Deployment Functions
 *
 * This section defines functions to be executed after package deployment.
 * These functions serve purposes such as:
 * 1. Updating chain configuration with newly deployed object IDs
 * 2. Submitting additional transactions for contract setup
 *
 * Define post-deployment functions for each supported package below.
 */

async function postDeployRelayerDiscovery(published, keypair, client, config, chain, options) {
    const [relayerDiscoveryObjectId, relayerDiscoveryObjectIdv0, ownerCap, upgradeCap] = getObjectIdsByObjectTypes(published.publishTxn, [
        `${published.packageId}::discovery::RelayerDiscovery`,
        `${published.packageId}::relayer_discovery_v0::RelayerDiscovery_v0`,
        `${published.packageId}::owner_cap::OwnerCap`,
        `${suiPackageAddress}::package::UpgradeCap`,
    ]);

    await broadcastRestrictedUpgradePolicy(client, keypair, upgradeCap, options);

    chain.contracts.RelayerDiscovery.objects = {
        RelayerDiscovery: relayerDiscoveryObjectId,
        RelayerDiscoveryv0: relayerDiscoveryObjectIdv0,
        OwnerCap: ownerCap,
        UpgradeCap: upgradeCap,
    };
}

async function postDeployUtils(published, keypair, client, config, chain, options) {
    const [upgradeCap] = getObjectIdsByObjectTypes(published.publishTxn, [`${suiPackageAddress}::package::UpgradeCap`]);

    await broadcastRestrictedUpgradePolicy(client, keypair, upgradeCap, options);

    if (options.policy !== 'immutable') {
        chain.contracts.Utils.objects = {
            UpgradeCap: upgradeCap,
        };
    }
}

async function postDeployVersionControl(published, keypair, client, config, chain, options) {
    const [upgradeCap] = getObjectIdsByObjectTypes(published.publishTxn, [`${suiPackageAddress}::package::UpgradeCap`]);

    await broadcastRestrictedUpgradePolicy(client, keypair, upgradeCap, options);

    if (options.policy !== 'immutable') {
        chain.contracts.VersionControl.objects = {
            UpgradeCap: upgradeCap,
        };
    }
}

async function postDeployAbi(published, keypair, client, config, chain, options) {
    const [upgradeCap] = getObjectIdsByObjectTypes(published.publishTxn, [`${suiPackageAddress}::package::UpgradeCap`]);

    await broadcastRestrictedUpgradePolicy(client, keypair, upgradeCap, options);

    if (options.policy !== 'immutable') {
        chain.contracts.Abi.objects = {
            UpgradeCap: upgradeCap,
        };
    }
}

async function postDeployGasService(published, keypair, client, config, chain, options) {
    const [OperatorCapObjectId, OwnerCapObjectId, gasServiceObjectId, gasServicev0ObjectId, upgradeCap] = getObjectIdsByObjectTypes(
        published.publishTxn,
        [
            `${published.packageId}::operator_cap::OperatorCap`,
            `${published.packageId}::owner_cap::OwnerCap`,
            `${published.packageId}::gas_service::GasService`,
            `${published.packageId}::gas_service_v0::GasService_v0`,
            `${suiPackageAddress}::package::UpgradeCap`,
        ],
    );

    await broadcastRestrictedUpgradePolicy(client, keypair, upgradeCap, options);

    chain.contracts.GasService.objects = {
        OperatorCap: OperatorCapObjectId,
        OwnerCap: OwnerCapObjectId,
        GasService: gasServiceObjectId,
        GasServicev0: gasServicev0ObjectId,
        UpgradeCap: upgradeCap,
    };
}

async function postDeployExample(published, keypair, client, config, chain, options) {
    const relayerDiscovery = chain.contracts.RelayerDiscovery?.objects?.RelayerDiscovery;
    const { policy } = options;

    // GMP Example Params
    const [gmpSingletonObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [`${published.packageId}::gmp::Singleton`]);

    // InterchainTokenService Example Params
    const itsObjectId = chain.contracts.InterchainTokenService?.objects?.InterchainTokenService;
    const [itsSingletonObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [`${published.packageId}::its::Singleton`]);

    const tx = new Transaction();

    tx.moveCall({
        target: `${published.packageId}::gmp::register_transaction`,
        arguments: [tx.object(relayerDiscovery), tx.object(gmpSingletonObjectId)],
    });

    tx.moveCall({
        target: `${published.packageId}::its::register_transaction`,
        arguments: [tx.object(relayerDiscovery), tx.object(itsSingletonObjectId), tx.object(itsObjectId), tx.object(suiClockAddress)],
    });

    const [upgradeCap] = getObjectIdsByObjectTypes(published.publishTxn, [`${suiPackageAddress}::package::UpgradeCap`]);
    restrictUpgradePolicy(tx, policy, upgradeCap);

    await broadcast(client, keypair, tx, 'Registered Transaction', options);

    const gmpChannelId = await getSingletonChannelId(client, gmpSingletonObjectId);
    const itsChannelId = await getSingletonChannelId(client, itsSingletonObjectId);

    chain.contracts.Example.objects = {
        GmpSingleton: gmpSingletonObjectId,
        GmpChannelId: gmpChannelId,
        ItsSingleton: itsSingletonObjectId,
        ItsChannelId: itsChannelId,
    };
}

async function postDeployOperators(published, keypair, client, config, chain, options) {
    const [operatorsObjectId, ownerCapObjectId, upgradeCap] = getObjectIdsByObjectTypes(published.publishTxn, [
        `${published.packageId}::operators::Operators`,
        `${published.packageId}::operators::OwnerCap`,
        `${suiPackageAddress}::package::UpgradeCap`,
    ]);

    await broadcastRestrictedUpgradePolicy(client, keypair, upgradeCap, options);

    chain.contracts.Operators.objects = {
        Operators: operatorsObjectId,
        OwnerCap: ownerCapObjectId,
        UpgradeCap: upgradeCap,
    };
}

async function postDeployAxelarGateway(published, keypair, client, config, chain, options) {
    const { packageId, publishTxn } = published;
    const { minimumRotationDelay, policy, previousSigners } = options;
    const operator = options.operator || keypair.toSuiAddress();
    const signers = await getSigners(keypair, config, chain, options);
    const domainSeparator = await getDomainSeparator(config.axelar, chain, options);

    validateParameters({
        isNonEmptyString: { previousSigners },
        isValidNumber: { minimumRotationDelay },
    });

    const [ownerCap, upgradeCap] = getObjectIdsByObjectTypes(publishTxn, [
        `${packageId}::owner_cap::OwnerCap`,
        `${suiPackageAddress}::package::UpgradeCap`,
    ]);

    const encodedSigners = bcsStructs.gateway.WeightedSigners.serialize({
        ...signers,
        nonce: bcsStructs.common.Bytes32.serialize(signers.nonce).toBytes(),
    }).toBytes();

    const tx = new Transaction();

    tx.moveCall({
        target: `${packageId}::gateway::setup`,
        arguments: [
            tx.object(ownerCap),
            tx.pure.address(operator),
            tx.pure.address(domainSeparator),
            tx.pure.u64(minimumRotationDelay),
            tx.pure.u64(options.previousSigners),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedSigners).toBytes()),
            tx.object(suiClockAddress),
        ],
    });

    restrictUpgradePolicy(tx, policy, upgradeCap);

    let result = await broadcast(client, keypair, tx, 'Setup Gateway', options);

    const maxRetries = 10;
    let retry = 0;

    while (result.objectChanges == undefined) {
        retry++;
        if (retry > maxRetries) {
            throw new Error(`failed to fetch object changes for tx ${result.digest}`);
        }

        result = await client.getTransactionBlock({
            digest: result.digest,
            options: {
                showEffects: true,
                showObjectChanges: true,
            },
        });
    }

    const [gateway, gatewayv0] = getObjectIdsByObjectTypes(result, [
        `${packageId}::gateway::Gateway`,
        `${packageId}::gateway_v0::Gateway_v0`,
    ]);

    // Update chain configuration
    chain.contracts.AxelarGateway = {
        ...chain.contracts.AxelarGateway,
        objects: {
            Gateway: gateway,
            UpgradeCap: upgradeCap,
            Gatewayv0: gatewayv0,
            OwnerCap: ownerCap,
        },
        domainSeparator,
        operator,
        minimumRotationDelay: minimumRotationDelay / 1000, // convert from milliseconds to seconds
    };
}

async function postDeployIts(published, keypair, client, config, chain, options) {
    const relayerDiscovery = chain.contracts.RelayerDiscovery?.objects?.RelayerDiscovery;

    const { chainName, policy } = options;

    const itsHubAddress = config.axelar.contracts.InterchainTokenService.address;

    const [ownerCapObjectId, creatorCapObjectId, operatorCapId, upgradeCapObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [
        `${published.packageId}::owner_cap::OwnerCap`,
        `${published.packageId}::creator_cap::CreatorCap`,
        `${published.packageId}::operator_cap::OperatorCap`,
        `${suiPackageAddress}::package::UpgradeCap`,
    ]);

    let tx = new Transaction();

    restrictUpgradePolicy(tx, policy, upgradeCapObjectId);

    tx.moveCall({
        target: `${published.packageId}::interchain_token_service::setup`,
        arguments: [tx.object(creatorCapObjectId), tx.pure.string(chainName), tx.pure.string(itsHubAddress)],
    });

    const setupReceipt = await broadcast(client, keypair, tx, 'Setup', options);

    const [InterchainTokenServiceObjectId, InterchainTokenServiceV0ObjectId] = getObjectIdsByObjectTypes(setupReceipt, [
        `${published.packageId}::interchain_token_service::InterchainTokenService`,
        `${published.packageId}::interchain_token_service_v0::InterchainTokenService_v0`,
    ]);
    await new Promise((resolve) => setTimeout(resolve, 2000));
    const channelId = await getItsChannelId(client, InterchainTokenServiceV0ObjectId);

    chain.contracts.InterchainTokenService.objects = {
        InterchainTokenService: InterchainTokenServiceObjectId,
        InterchainTokenServicev0: InterchainTokenServiceV0ObjectId,
        ChannelId: channelId,
        OwnerCap: ownerCapObjectId,
        OperatorCap: operatorCapId,
        UpgradeCap: upgradeCapObjectId,
    };

    tx = new Transaction();
    tx.moveCall({
        target: `${published.packageId}::discovery::register_transaction`,
        arguments: [tx.object(InterchainTokenServiceObjectId), tx.object(relayerDiscovery)],
    });

    await broadcast(client, keypair, tx, 'Registered Transaction', options);
}

async function deploy(keypair, client, supportedContract, config, chain, options) {
    const { packageDir, packageName } = supportedContract;

    // Print warning if version mismatch from defined version in version.json
    checkSuiVersionMatch();

    // Check if dependencies are deployed
    const dependencies = getLocalDependencies(packageDir, `${__dirname}/../node_modules/@axelar-network/axelar-cgp-sui/move`);

    for (const { name } of dependencies) {
        if (!chain.contracts[name]) {
            throw new Error(`Contract ${name} needed to be deployed before deploying ${packageName}`);
        }
    }

    // Deploy package
    const published = await deployPackage(packageDir, client, keypair, options);

    printInfo(`${packageName} Package ID`, published.packageId);

    // Update chain configuration with deployed contract address
    chain.contracts[packageName] = {
        address: published.packageId,
        versions: {
            0: published.packageId,
        },
        deployer: keypair.toSuiAddress(),
    };

    chain.contracts[packageName].structs = await getStructs(client, published.packageId);

    // Execute post-deployment function
    const executePostDeploymentFn = PACKAGE_CONFIGS.postDeployFunctions[packageName];

    if (executePostDeploymentFn) {
        await executePostDeploymentFn(published, keypair, client, config, chain, options);
    }

    printInfo(`${packageName} Configuration Updated`, JSON.stringify(chain.contracts[packageName], null, 2));
}

async function upgrade(keypair, client, supportedPackage, policy, config, chain, options) {
    const { packageName, packageDir } = supportedPackage;
    options.policy = policy;

    if (!chain.contracts[packageName]) {
        throw new Error(`Cannot find specified contract: ${packageName}`);
    }

    const contractsConfig = chain.contracts;
    const contractConfig = contractsConfig?.[packageName];

    validateParameters({ isNonEmptyString: { packageName } });

    const packageDependencies = getLocalDependencies(packageDir, moveDir);

    for (const { name } of packageDependencies) {
        const packageAddress = contractsConfig[name]?.address;
        const version = Math.max(0, Object.keys(contractsConfig[name]?.versions || {}).length - 1);
        const legacyPackageId = version > 0 ? contractsConfig[name]?.versions['0'] : undefined;

        let network;
        switch (options.env) {
            case 'devnet':
            case 'testnet':
            case 'mainnet': {
                network = options.env;
                break;
            }
            default: {
                network = 'testnet';
            }
        }

        updateMoveToml(packageDir, packageAddress, moveDir, undefined, version, network, legacyPackageId);
    }

    const builder = new TxBuilder(client);
    const result = await upgradePackage(client, keypair, supportedPackage, contractConfig, builder, options);

    if (!options.offline) {
        // The new upgraded package takes a bit of time to register, so we wait.
        await new Promise((resolve) => setTimeout(resolve, 1000));
        chain.contracts[packageName].structs = await getStructs(client, result.packageId);
    }
}

async function migrate(keypair, client, supportedPackage, config, chain, options) {
    const { packageName } = supportedPackage;

    validateParameters({
        // Contract
        isNonArrayObject: { contractEntry: chain.contracts[packageName] },
        isNonEmptyString: { contractAddress: chain.contracts[packageName].address },
        // OwnerCap
        isNonArrayObject: { ownerEntry: chain.contracts[packageName].objects },
        isNonEmptyString: { ownerAddress: chain.contracts[packageName].objects.OwnerCap },
    });
    const contractConfig = chain.contracts[packageName];
    const ownerCap = contractConfig.objects.OwnerCap;

    const builder = new TxBuilder(client);

    switch (packageName) {
        case 'AxelarGateway': {
            const result = await GatewayCli.migrate(keypair, client, config, chain, contractConfig, null, options);
            return await broadcast(client, keypair, result.tx, result.message, options);
        }
        case 'InterchainTokenService': {
            const InterchainTokenService = contractConfig.objects.InterchainTokenService;
            const RelayerDiscovery = chain.contracts.RelayerDiscovery.objects.RelayerDiscovery;

            if (typeof InterchainTokenService !== 'string') throw new Error(`Cannot find object of specified contract: ${packageName}`);

            await builder.moveCall({
                target: `${contractConfig.address}::interchain_token_service::migrate`,
                arguments: [InterchainTokenService, ownerCap],
            });

            await builder.moveCall({
                target: `${contractConfig.address}::discovery::register_transaction`,
                arguments: [InterchainTokenService, RelayerDiscovery],
            });

            break;
        }
        default: {
            throw new Error(`Post-upgrade migration not supported for ${packageName}`);
        }
    }

    if (packageName !== 'AxelarGateway') await broadcastFromTxBuilder(builder, keypair, `Migrate Package ${packageName}`, options);
}

async function syncPackages(keypair, client, config, chain, options) {
    // Remove the move directory and its contents if it exists
    fs.rmSync(moveDir, { recursive: true, force: true });

    for (const packageDir of PACKAGE_DIRS) {
        copyMovePackage(packageDir, null, moveDir);
        const packageName = readMovePackageName(packageDir);
        const packageId = chain.contracts[packageName]?.address;

        let network;
        switch (options.env) {
            case 'devnet':
            case 'testnet':
            case 'mainnet': {
                network = options.env;
                break;
            }
            default: {
                network = 'testnet';
            }
        }

        if (!packageId) {
            printWarn(`Package ID for ${packageName} not found in config. Skipping...`);
            continue;
        }

        const version = Math.max(0, Object.keys(chain.contracts[packageName]?.versions || {}).length - 1);
        const legacyPackageId = version > 0 ? chain.contracts[packageName]?.versions['0'] : undefined;

        updateMoveToml(packageDir, packageId, moveDir, undefined, version, network, legacyPackageId);
        printInfo(`Synced ${packageName} with package ID`, packageId);
    }
}

async function mainProcessor(args, options, processor) {
    const config = loadConfig(options.env);
    const sui = getChainConfig(config.chains, options.chainName);
    const [keypair, client] = getWallet(sui, options);

    printInfo('Environment', options.env);
    printInfo('Chain Name', options.chainName);
    await printWalletInfo(keypair, client, sui, options);

    await processor(keypair, client, ...args, config, sui, options);

    saveConfig(config, options.env);
}

/**
 * Command Options
 *
 * This section defines options for the command that are specific to each package.
 */

// Common deploy command options for all packages
const DEPLOY_CMD_OPTIONS = [
    new Option('--policy <policy>', 'upgrade policy for upgrade cap: For example, use "any_upgrade" to allow all types of upgrades')
        .choices(['immutable', 'any_upgrade', 'code_upgrade', 'dep_upgrade'])
        .default('any_upgrade'),
];

// Gateway deploy command options
const GATEWAY_CMD_OPTIONS = [
    new Option('--signers <signers>', 'JSON with the initial signer set').env('SIGNERS'),
    new Option('--operator <operator>', 'operator for the gateway (defaults to the deployer address)').env('OPERATOR'),
    new Option('--minimumRotationDelay <minimumRotationDelay>', 'minium delay for signer rotations (in second)')
        .argParser((val) => parseInt(val) * 1000)
        .default(24 * 60 * 60),
    new Option(
        '--domainSeparator <domainSeparator>',
        'domain separator (pass in the keccak256 hash value OR "offline" meaning that its computed locally)',
    ).default('offline'),
    new Option('--nonce <nonce>', 'nonce for the signer (defaults to HashZero)'),
    new Option('--previousSigners <previousSigners>', 'number of previous signers to retain').default('15'),
];

const addDeployOptions = (program) => {
    // Get the package name from the program name
    const packageName = program.name();
    // Find the corresponding options for the package
    const cmdOptions = PACKAGE_CONFIGS.cmdOptions[packageName];

    if (cmdOptions) {
        const options = cmdOptions();
        // Add the options to the program
        options.forEach((option) => program.addOption(option));
    }

    // Add the base deploy options to the program
    DEPLOY_CMD_OPTIONS.forEach((option) => program.addOption(option));

    return program;
};

if (require.main === module) {
    // 1st level command
    const program = new Command('deploy-contract').description('Deploy/Upgrade packages');

    // 2nd level commands
    const deployCmd = new Command('deploy').description('Deploy a Sui package');
    const upgradeCmd = new Command('upgrade').description('Upgrade a Sui package');
    const migrateCmd = new Command('migrate').description('Migrate a Sui package after upgrading');

    // 3rd level commands for `deploy`
    const deployContractCmds = supportedPackages.map((supportedPackage) => {
        const { packageName } = supportedPackage;
        const command = new Command(packageName).description(`Deploy ${packageName} contract`);

        return addDeployOptions(command).action((options) => {
            mainProcessor([supportedPackage], options, deploy);
        });
    });

    // Add 3rd level commands to 2nd level command `deploy`
    deployContractCmds.forEach((cmd) => deployCmd.addCommand(cmd));

    // 3rd level commands for `upgrade`
    const upgradeContractCmds = supportedPackages.map((supportedPackage) => {
        const { packageName } = supportedPackage;
        return new Command(packageName)
            .description(`Upgrade ${packageName} contract`)
            .command(`${packageName} <policy>`)
            .addOption(new Option('--sender <sender>', 'transaction sender'))
            .addOption(new Option('--digest <digest>', 'digest hash for upgrade'))
            .addOption(new Option('--offline', 'store tx block for sign'))
            .addOption(new Option('--txFilePath <file>', 'unsigned transaction will be stored'))
            .action((policy, options) => {
                mainProcessor([supportedPackage, policy], options, upgrade);
            });
    });

    // 3rd level commands for `migrate`
    const migrateContractCmds = supportedPackages.map((supportedPackage) => {
        const { packageName } = supportedPackage;
        return new Command(packageName)
            .description(`Migrate ${packageName} contract after upgrade`)
            .command(`${packageName}`)
            .addOption(new Option('--migrate-data <migrateData>', 'bcs encoded data to pass to the migrate function'))
            .addOption(new Option('--sender <sender>', 'transaction sender'))
            .addOption(new Option('--digest <digest>', 'digest hash for upgrade'))
            .addOption(new Option('--offline', 'store tx block for sign'))
            .addOption(new Option('--txFilePath <file>', 'unsigned transaction will be stored'))
            .action((options) => {
                mainProcessor([supportedPackage], options, migrate);
            });
    });

    const syncCmd = new Command('sync').description('Sync local Move packages with deployed addresses').action((options) => {
        mainProcessor([], options, syncPackages);
    });

    // Add 3rd level commands to 2nd level command `upgrade`
    upgradeContractCmds.forEach((cmd) => upgradeCmd.addCommand(cmd));

    // Add 3rd level commands to 2nd level command `migrate`
    migrateContractCmds.forEach((cmd) => migrateCmd.addCommand(cmd));

    // Add base options to all 2nd and 3rd level commands
    addOptionsToCommands(deployCmd, addBaseOptions);
    addOptionsToCommands(upgradeCmd, addBaseOptions);
    addOptionsToCommands(migrateCmd, addBaseOptions);
    addBaseOptions(syncCmd);

    // Add 2nd level commands to 1st level command
    program.addCommand(deployCmd);
    program.addCommand(upgradeCmd);
    program.addCommand(migrateCmd);
    program.addCommand(syncCmd);

    program.parse();
}
