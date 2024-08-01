const { Command, Option } = require('commander');
const { updateMoveToml, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { ethers } = require('hardhat');
const { toB64 } = require('@mysten/sui/utils');
const { bcs } = require('@mysten/sui/bcs');
const { Transaction } = require('@mysten/sui/transactions');
const {
    utils: { arrayify },
} = ethers;
const { saveConfig, printInfo, validateParameters, writeJSON, getDomainSeparator } = require('../common');
const { addBaseOptions, addOptionsToCommands } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { bytes32Struct, signersStruct } = require('./types-utils');
const { upgradePackage, UPGRADE_POLICIES } = require('./deploy-utils');
const {
    loadSuiConfig,
    getSigners,
    deployPackage,
    getObjectIdsByObjectTypes,
    suiPackageAddress,
    suiClockAddress,
    readMovePackageName,
    getChannelId,
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
const PACKAGE_DIRS = ['gas_service', 'test', 'axelar_gateway', 'operators'];

/**
 * Post-Deployment Functions Mapping
 *
 * This object maps each package name to a post-deployment function.
 */
const POST_DEPLOY_FUNCTIONS = {
    GasService: postDeployGasService,
    Test: postDeployTest,
    Operators: postDeployOperators,
    AxelarGateway: postDeployAxelarGateway,
};

/**
 * Command Options Mapping
 *
 * This object maps each package name to a function that returns an array of command options.
 */
const CMD_OPTIONS = {
    AxelarGateway: () => [...DEPLOY_CMD_OPTIONS, ...GATEWAY_CMD_OPTIONS],
    GasService: () => DEPLOY_CMD_OPTIONS,
    Test: () => DEPLOY_CMD_OPTIONS,
    Operators: () => DEPLOY_CMD_OPTIONS,
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

async function postDeployGasService(published, args) {
    const { chain } = args;
    const [gasCollectorCapObjectId, gasServiceObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [
        `${published.packageId}::gas_service::GasCollectorCap`,
        `${published.packageId}::gas_service::GasService`,
    ]);
    chain.contracts.GasService.objects = {
        GasCollectorCap: gasCollectorCapObjectId,
        GasService: gasServiceObjectId,
    };
}

async function postDeployTest(published, args) {
    const { chain, config, options } = args;
    const [keypair, client] = getWallet(chain, options);
    const relayerDiscovery = config.sui.contracts.AxelarGateway?.objects?.RelayerDiscovery;

    const [singletonObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [`${published.packageId}::test::Singleton`]);
    const channelId = await getChannelId(client, singletonObjectId);
    chain.contracts.Test.objects = { Singleton: singletonObjectId, ChannelId: channelId };

    const tx = new Transaction();
    tx.moveCall({
        target: `${published.packageId}::test::register_transaction`,
        arguments: [tx.object(relayerDiscovery), tx.object(singletonObjectId)],
    });

    const registerTx = await broadcast(client, keypair, tx);

    printInfo('Register transaction', registerTx.digest);
}

async function postDeployOperators(published, args) {
    const { chain } = args;
    const [operatorsObjectId, ownerCapObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [
        `${published.packageId}::operators::Operators`,
        `${published.packageId}::operators::OwnerCap`,
    ]);
    chain.contracts.Operators.objects = {
        Operators: operatorsObjectId,
        OwnerCap: ownerCapObjectId,
    };
}

async function postDeployAxelarGateway(published, args) {
    const { keypair, client, config, chain, options } = args;
    const { packageId, publishTxn } = published;
    const { minimumRotationDelay, policy, previousSigners } = options;
    const operator = options.operator || keypair.toSuiAddress();
    const signers = await getSigners(keypair, config, chain, options);
    const domainSeparator = await getDomainSeparator(config, chain, options);

    validateParameters({
        isNonEmptyString: { previousSigners },
        isValidNumber: { minimumRotationDelay },
    });

    const [creatorCap, relayerDiscovery, upgradeCap] = getObjectIdsByObjectTypes(publishTxn, [
        `${packageId}::gateway::CreatorCap`,
        `${packageId}::discovery::RelayerDiscovery`,
        `${suiPackageAddress}::package::UpgradeCap`,
    ]);

    const encodedSigners = signersStruct
        .serialize({
            ...signers,
            nonce: bytes32Struct.serialize(signers.nonce).toBytes(),
        })
        .toBytes();

    const tx = new Transaction();

    const separator = tx.moveCall({
        target: `${packageId}::bytes32::new`,
        arguments: [tx.pure(arrayify(domainSeparator))],
    });

    tx.moveCall({
        target: `${packageId}::gateway::setup`,
        arguments: [
            tx.object(creatorCap),
            tx.pure.address(operator),
            separator,
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

    const result = await broadcast(client, keypair, tx);

    printInfo('Setup Gateway', result.digest);

    const [gateway] = getObjectIdsByObjectTypes(result, [`${packageId}::gateway::Gateway`]);

    // Update chain configuration
    chain.contracts.AxelarGateway = {
        objects: {
            Gateway: gateway,
            RelayerDiscovery: relayerDiscovery,
            UpgradeCap: upgradeCap,
        },
        domainSeparator,
        operator,
        minimumRotationDelay,
    };
}

async function deploy(keypair, client, supportedContract, config, chain, options) {
    const { packageDir, packageName } = supportedContract;

    // Deploy package
    const published = await deployPackage(packageDir, client, keypair, options);

    printInfo(`Deployed ${packageName}`, published.publishTxn.digest);

    // Update chain configuration with deployed contract address
    chain.contracts[packageName] = {
        address: published.packageId,
    };

    // Execute post-deployment function
    const executePostDeploymentFn = POST_DEPLOY_FUNCTIONS[packageName];
    executePostDeploymentFn(published, { keypair, client, config, chain, options });

    printInfo(`${packageName} Configuration Updated`, JSON.stringify(chain.contracts[packageName], null, 2));
}

async function upgrade(keypair, client, supportedPackage, policy, config, chain, options) {
    const { packageDependencies } = options;
    const { packageName } = supportedPackage;
    options.policy = policy;

    if (!chain.contracts[packageName]) {
        throw new Error(`Cannot find specified contract: ${packageName}`);
    }

    const contractsConfig = chain.contracts;
    const contractConfig = contractsConfig?.[packageName];

    validateParameters({ isNonEmptyString: { packageName } });

    if (packageDependencies) {
        for (const dependencies of packageDependencies) {
            const packageId = contractsConfig[dependencies]?.address;
            updateMoveToml(dependencies, packageId);
        }
    }

    const builder = new TxBuilder(client);
    await upgradePackage(client, keypair, supportedPackage, contractConfig, builder, options);
}

async function mainProcessor(args, options, processor) {
    const config = loadSuiConfig(options.env);
    const [keypair, client] = getWallet(config.sui, options);
    await printWalletInfo(keypair, client, config.sui, options);
    await processor(keypair, client, ...args, config, config.sui, options);
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
    new Option('--domainSeparator <domainSeparator>', 'domain separator'),
    new Option('--nonce <nonce>', 'nonce for the signer (defaults to HashZero)'),
    new Option('--previousSigners <previousSigners>', 'number of previous signers to retain').default('15'),
];

const addDeployOptions = (program) => {
    // Get the package name from the program name
    const packageName = program.name();

    // Find the corresponding options for the package
    const options = CMD_OPTIONS[packageName]();

    // Add the options to the program
    options.forEach((option) => program.addOption(option));

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
            .description(`Deploy ${packageName} contract`)
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
