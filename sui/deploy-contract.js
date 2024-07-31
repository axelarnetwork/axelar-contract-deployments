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
const { addBaseOptions, addDeployOptions, addOptionsToCommands } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { bytes32Struct, signersStruct } = require('./types-utils');
const { upgradePackage } = require('./deploy-utils');
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

// A list of currently supported packages which are the folder names in `node_modules/@axelar-network/axelar-cgp-sui/move`
const supportedPackageDirs = ['gas_service', 'test', 'axelar_gateway'];

// Map supported packages to their package names and directories
const supportedPackages = supportedPackageDirs.map((dir) => ({
    packageName: readMovePackageName(dir),
    packageDir: dir,
}));

/** ######## Post Deployment Functions ######## **/
// Define the post deployment functions for each supported package here. These functions should be called after the package is deployed.
// Use cases include:
// 1. Update the chain config with deployed object ids
// 2. Submit additional transactions to setup the contracts.

async function postDeployGasService(published, chain) {
    const [gasCollectorCapObjectId, gasServiceObjectId] = getObjectIdsByObjectTypes(published.publishTxn, [
        `${published.packageId}::gas_service::GasCollectorCap`,
        `${published.packageId}::gas_service::GasService`,
    ]);
    chain.contracts.GasService.objects = {
        GasCollectorCap: gasCollectorCapObjectId,
        GasService: gasServiceObjectId,
    };
}

async function postDeployTest(published, config, chain, options) {
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
        const upgradeType = policy === 'code_upgrade' ? 'only_additive_upgrades' : 'only_dep_upgrades';

        tx.moveCall({
            target: `${suiPackageAddress}::package::${upgradeType}`,
            arguments: [tx.object(upgradeCap)],
        });
    }

    const result = await broadcast(client, keypair, tx);

    printInfo('Setup transaction digest', result.digest);

    const [gateway] = getObjectIdsByObjectTypes(result, [`${packageId}::gateway::Gateway`]);

    const contractConfig = chain.contracts.AxelarGateway;

    contractConfig.objects = {
        Gateway: gateway,
        RelayerDiscovery: relayerDiscovery,
        UpgradeCap: upgradeCap,
    };
    contractConfig.domainSeparator = domainSeparator;
    contractConfig.operator = operator;
    contractConfig.minimumRotationDelay = minimumRotationDelay;
}

async function deploy(keypair, client, supportedContract, config, chain, options) {
    const { packageDir, packageName } = supportedContract;

    if (!chain.contracts[packageName]) {
        chain.contracts[packageName] = {};
    }

    const published = await deployPackage(packageDir, client, keypair, options);

    printInfo(`Deployed ${packageName}`, published.publishTxn.digest);

    if (!chain.contracts[packageName]) {
        chain.contracts[packageName] = {};
    }

    switch (packageName) {
        case 'GasService':
            await postDeployGasService(published, chain);
            break;
        case 'AxelarGateway':
            await postDeployAxelarGateway(published, keypair, client, config, chain, options);
            break;
        case 'Test':
            await postDeployTest(published, config, chain, options);
            break;
        default:
            throw new Error(`${packageName} is not supported.`);
    }

    printInfo(`${packageName} deployed`, JSON.stringify(chain.contracts[packageName], null, 2));
}

async function upgrade(keypair, client, contractName, policy, config, chain, options) {
    const { packageDependencies } = options;
    options.policy = policy;

    if (!chain.contracts[contractName]) {
        throw new Error(`Cannot find specified contract: ${contractName}`);
    }

    const contractsConfig = chain.contracts;
    const packageConfig = contractsConfig?.[contractName];

    validateParameters({ isNonEmptyString: { contractName } });

    if (packageDependencies) {
        for (const dependencies of packageDependencies) {
            const packageId = contractsConfig[dependencies]?.address;
            updateMoveToml(dependencies, packageId);
        }
    }

    const builder = new TxBuilder(client);
    await upgradePackage(client, keypair, contractName, packageConfig, builder, options);
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

if (require.main === module) {
    // 1st level command
    const program = new Command('deploy-contract').description('Deploy/Upgrade packages');

    // 2nd level commands
    const deployCmd = new Command('deploy');
    const upgradeCmd = new Command('upgrade');

    // 3rd level commands
    const deployContractCmds = supportedPackages.map((supportedPackage) => {
        const { packageName } = supportedPackage;
        const command = new Command(packageName).description(`Deploy ${packageName} contract`);

        return addDeployOptions(command).action((options) => {
            mainProcessor([supportedPackage], options, deploy);
        });
    });

    // Add 3rd level commands to 2nd level command `deploy`
    deployContractCmds.forEach((cmd) => deployCmd.addCommand(cmd));

    // Add base options to all 2nd and 3rd level commands
    addOptionsToCommands(deployCmd, addBaseOptions);
    addBaseOptions(upgradeCmd);

    // Define options for 2nd level command `upgrade`
    upgradeCmd
        .description('Upgrade a Sui package')
        .command('upgrade <packageName> <policy>')
        .addOption(new Option('--sender <sender>', 'transaction sender'))
        .addOption(new Option('--digest <digest>', 'digest hash for upgrade'))
        .addOption(new Option('--offline', 'store tx block for sign'))
        .addOption(new Option('--txFilePath <file>', 'unsigned transaction will be stored'))
        .action((packageName, policy, options) => {
            mainProcessor([packageName, policy], options, upgrade);
        });

    // Add 2nd level commands to 1st level command
    program.addCommand(deployCmd);
    program.addCommand(upgradeCmd);

    program.parse();
}
