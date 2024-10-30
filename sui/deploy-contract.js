const { Command, Option } = require('commander');
const { getLocalDependencies, updateMoveToml, TxBuilder, bcsStructs } = require('@axelar-network/axelar-cgp-sui');
const { toB64 } = require('@mysten/sui/utils');
const { bcs } = require('@mysten/sui/bcs');
const { Transaction } = require('@mysten/sui/transactions');
const { saveConfig, printInfo, validateParameters, writeJSON, getDomainSeparator, loadConfig, getChainConfig } = require('../common');
const {
    addBaseOptions,
    addOptionsToCommands,
    getWallet,
    printWalletInfo,
    broadcast,
    upgradePackage,
    UPGRADE_POLICIES,
    getSigners,
    deployPackage,
    getObjectIdsByObjectTypes,
    suiPackageAddress,
    suiClockAddress,
    readMovePackageName,
    getSingletonChannelId,
    getItsChannelId,
    getSquidChannelId,
    checkSuiVersionMatch,
    moveDir,
} = require('./utils');

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
    'its',
    'squid',
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
        ITS: postDeployIts,
        Squid: postDeploySquid,
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
    const [relayerDiscoveryObjectId, relayerDiscoveryObjectIdv0] = getObjectIdsByObjectTypes(published.publishTxn, [
        `${published.packageId}::discovery::RelayerDiscovery`,
        `${published.packageId}::relayer_discovery_v0::RelayerDiscovery_v0`,
    ]);

    chain.contracts.RelayerDiscovery.objects = {
        RelayerDiscovery: relayerDiscoveryObjectId,
        RelayerDiscoveryv0: relayerDiscoveryObjectIdv0,
    };
}

async function postDeployGasService(published, keypair, client, config, chain, options) {
    const [gasCollectorCapObjectId, gasServiceObjectId, gasServicev0ObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [
        `${published.packageId}::gas_service::GasCollectorCap`,
        `${published.packageId}::gas_service::GasService`,
        `${published.packageId}::gas_service_v0::GasService_v0`,
    ]);
    chain.contracts.GasService.objects = {
        GasCollectorCap: gasCollectorCapObjectId,
        GasService: gasServiceObjectId,
        GasServicev0: gasServicev0ObjectId,
    };
}

async function postDeployExample(published, keypair, client, config, chain, options) {
    const relayerDiscovery = chain.contracts.RelayerDiscovery?.objects?.RelayerDiscovery;

    // GMP Example Params
    const [gmpSingletonObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [`${published.packageId}::gmp::Singleton`]);

    // ITS Example Params
    const itsObjectId = chain.contracts.ITS?.objects?.ITS;
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

    await broadcast(client, keypair, tx, 'Registered Transaction');

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
    const [operatorsObjectId, ownerCapObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [
        `${published.packageId}::operators::Operators`,
        `${published.packageId}::operators::OwnerCap`,
    ]);
    chain.contracts.Operators.objects = {
        Operators: operatorsObjectId,
        OwnerCap: ownerCapObjectId,
    };
}

async function postDeployAxelarGateway(published, keypair, client, config, chain, options) {
    const { packageId, publishTxn } = published;
    const { minimumRotationDelay, policy, previousSigners } = options;
    const operator = options.operator || keypair.toSuiAddress();
    const signers = await getSigners(keypair, config, chain, options);
    const domainSeparator = await getDomainSeparator(config, chain, options);

    validateParameters({
        isNonEmptyString: { previousSigners },
        isValidNumber: { minimumRotationDelay },
    });

    const [creatorCap, upgradeCap] = getObjectIdsByObjectTypes(publishTxn, [
        `${packageId}::gateway::CreatorCap`,
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
            tx.object(creatorCap),
            tx.pure.address(operator),
            tx.pure.address(domainSeparator),
            tx.pure.u64(minimumRotationDelay),
            tx.pure.u64(options.previousSigners),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedSigners).toBytes()),
            tx.object(suiClockAddress),
        ],
    });

    if (policy !== 'any_upgrade') {
        const upgradeType = UPGRADE_POLICIES[policy];
        tx.moveCall({
            target: `${suiPackageAddress}::package::${upgradeType}`,
            arguments: [tx.object(upgradeCap)],
        });
    }

    const result = await broadcast(client, keypair, tx, 'Setup Gateway');

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
        },
        domainSeparator,
        operator,
        minimumRotationDelay,
    };
}

async function postDeployIts(published, keypair, client, config, chain, options) {
    const relayerDiscovery = chain.contracts.RelayerDiscovery?.objects?.RelayerDiscovery;

    const [itsObjectId, itsv0ObjectId, ownerCapObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [
        `${published.packageId}::its::ITS`,
        `${published.packageId}::its_v0::ITS_v0`,
        `${published.packageId}::owner_cap::OwnerCap`,
    ]);

    const channelId = await getItsChannelId(client, itsv0ObjectId);

    chain.contracts.ITS.objects = { ITS: itsObjectId, ITSv0: itsv0ObjectId, ChannelId: channelId, OwnerCap: ownerCapObjectId };

    const tx = new Transaction();
    tx.moveCall({
        target: `${published.packageId}::discovery::register_transaction`,
        arguments: [tx.object(itsObjectId), tx.object(relayerDiscovery)],
    });

    await broadcast(client, keypair, tx, 'Registered Transaction');
}

async function postDeploySquid(published, keypair, client, config, chain, options) {
    const relayerDiscovery = chain.contracts.RelayerDiscovery?.objects?.RelayerDiscovery;

    const [squidObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [`${published.packageId}::squid::Squid`]);
    const channelId = await getSquidChannelId(client, squidObjectId);
    chain.contracts.Squid.objects = { Squid: squidObjectId, ChannelId: channelId };

    const tx = new Transaction();
    tx.moveCall({
        target: `${published.packageId}::discovery::register_transaction`,
        arguments: [tx.object(squidObjectId), tx.object(chain.contracts.ITS.objects.ITS), tx.object(relayerDiscovery)],
    });

    await broadcast(client, keypair, tx, 'Registered Transaction');
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

    printInfo(`Deployed ${packageName} Package`, published.packageId);
    printInfo(`Deployed ${packageName} Tx`, published.publishTxn.digest);

    // Update chain configuration with deployed contract address
    chain.contracts[packageName] = {
        address: published.packageId,
    };

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
        updateMoveToml(packageDir, packageAddress, moveDir);
    }

    const builder = new TxBuilder(client);
    await upgradePackage(client, keypair, supportedPackage, contractConfig, builder, options);
}

async function mainProcessor(args, options, processor) {
    const config = loadConfig(options.env);
    const sui = getChainConfig(config, options.chainName);
    const [keypair, client] = getWallet(sui, options);

    printInfo('Environment', options.env);
    printInfo('Chain Name', options.chainName);
    await printWalletInfo(keypair, client, sui, options);

    await processor(keypair, client, ...args, config, sui, options);

    saveConfig(config, options.env);

    if (options.offline) {
        const { txFilePath } = options;
        validateParameters({ isNonEmptyString: { txFilePath } });

        const txB64Bytes = toB64(options.txBytes);

        writeJSON({ message: options.offlineMessage, status: 'PENDING', unsignedTx: txB64Bytes }, txFilePath);
        printInfo(`Unsigned transaction`, txFilePath);
    }
}

/**
 * Command Options
 *
 * This section defines options for the command that are specific to each package.
 */

// Common deploy command options for all packages
const DEPLOY_CMD_OPTIONS = [
    new Option('--policy <policy>', 'upgrade policy for upgrade cap: For example, use "any_upgrade" to allow all types of upgrades')
        .choices(['any_upgrade', 'code_upgrade', 'dep_upgrade'])
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

    // Add 3rd level commands to 2nd level command `upgrade`
    upgradeContractCmds.forEach((cmd) => upgradeCmd.addCommand(cmd));

    // Add base options to all 2nd and 3rd level commands
    addOptionsToCommands(deployCmd, addBaseOptions);
    addOptionsToCommands(upgradeCmd, addBaseOptions);

    // Add 2nd level commands to 1st level command
    program.addCommand(deployCmd);
    program.addCommand(upgradeCmd);

    program.parse();
}
