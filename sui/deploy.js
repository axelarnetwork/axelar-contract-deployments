const { Command, Option } = require('commander');
const { updateMoveToml, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { ethers } = require('hardhat');
const { toB64 } = require('@mysten/sui/utils');
const { bcs } = require('@mysten/sui/bcs');
const { Transaction } = require('@mysten/sui/transactions');
const {
    utils: { arrayify, hexlify, toUtf8Bytes, keccak256 },
    constants: { HashZero },
} = ethers;
const { saveConfig, printInfo, validateParameters, writeJSON } = require('../evm/utils');
const { addBaseOptions, addDeployOptions, addOptionsToCommands } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { loadSuiConfig, getAmplifierSigners, deployPackage, getObjectIdsByObjectTypes } = require('./utils');
const { bytes32Struct, signersStruct } = require('./types-utils');
const { upgradePackage } = require('./deploy-utils');
const { suiPackageAddress, suiClockAddress, readMovePackageName } = require('./utils');

// A list of currently supported packages which are the folder names in `node_modules/@axelar-network/axelar-cgp-sui/move`
const supportedPackageDirs = ['gas_service', 'test', 'axelar_gateway'];

// Map supported packages to their package names and directories
const supportedPackages = supportedPackageDirs.map((dir) => ({
    packageName: readMovePackageName(dir),
    packageDir: dir,
}));

async function getSigners(keypair, config, chain, options) {
    if (options.signers === 'wallet') {
        const pubKey = keypair.getPublicKey().toRawBytes();
        printInfo('Using wallet pubkey as the signer for the gateway', hexlify(pubKey));

        if (keypair.getKeyScheme() !== 'Secp256k1') {
            throw new Error('Only Secp256k1 pubkeys are supported by the gateway');
        }

        return {
            signers: [{ pub_key: pubKey, weight: 1 }],
            threshold: 1,
            nonce: options.nonce ? keccak256(toUtf8Bytes(options.nonce)) : HashZero,
        };
    } else if (options.signers) {
        printInfo('Using provided signers', options.signers);

        const signers = JSON.parse(options.signers);
        return {
            signers: signers.signers.map(({ pub_key: pubKey, weight }) => {
                return { pub_key: arrayify(pubKey), weight };
            }),
            threshold: signers.threshold,
            nonce: arrayify(signers.nonce) || HashZero,
        };
    }

    return getAmplifierSigners(config, chain);
}

async function deploy(keypair, client, contractName, config, chain, options) {
    if (!chain.contracts[contractName]) {
        chain.contracts[contractName] = {};
    }

    const { packageId, publishTxn } = await deployPackage(contractName, client, keypair, options);

    printInfo('Publish transaction digest: ', publishTxn.digest);

    const contractConfig = chain.contracts[contractName];
    contractConfig.address = packageId;
    contractConfig.objects = {};

    switch (contractName) {
        case 'gas_service': {
            const [GasService, GasCollectorCap] = getObjectIdsByObjectTypes(publishTxn, [
                `${packageId}::gas_service::GasService`,
                `${packageId}::gas_service::GasCollectorCap`,
            ]);
            contractConfig.objects = { GasService, GasCollectorCap };
            break;
        }

        case 'axelar_gateway': {
            const { minimumRotationDelay, domainSeparator, policy, previousSigners } = options;
            const operator = options.operator || keypair.toSuiAddress();
            const signers = await getSigners(keypair, config, chain, options);

            validateParameters({ isNonEmptyString: { previousSigners, minimumRotationDelay }, isKeccak256Hash: { domainSeparator } });

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

            contractConfig.objects = {
                gateway,
                relayerDiscovery,
                upgradeCap,
            };
            contractConfig.domainSeparator = domainSeparator;
            contractConfig.operator = operator;
            contractConfig.minimumRotationDelay = minimumRotationDelay;
            break;
        }

        default: {
            throw new Error(`${contractName} is not supported.`);
        }
    }

    printInfo(`${contractName} deployed`, JSON.stringify(chain.contracts[contractName], null, 2));
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
    const program = new Command("deploy-contract").description('Deploy/Upgrade packages');

    // 2nd level commands
    const deployCmd = new Command('deploy')
    const upgradeCmd = new Command('upgrade')

    // 3rd level commands
    const deployContractCmds = supportedPackages.map(({ packageName }) => {
        const command = new Command(packageName)
          .description(`Deploy ${packageName} contract`)

        return addDeployOptions(command)
          .action((options) => {
              mainProcessor([packageName], options, deploy);
          })
    });

    // Add 3rd level commands to 2nd level command `deploy`
    deployContractCmds.forEach((cmd) => deployCmd.addCommand(cmd));

    // Add base options to all 2nd and 3rd level commands
    addOptionsToCommands(deployCmd, addBaseOptions)
    addBaseOptions(upgradeCmd)

    // Define options for 2nd level command `upgrade`
    upgradeCmd
        .description('Upgrade a Sui package')
        .command('upgrade <contractName> <policy>')
        .addOption(new Option('--sender <sender>', 'transaction sender'))
        .addOption(new Option('--digest <digest>', 'digest hash for upgrade'))
        .addOption(new Option('--offline', 'store tx block for sign'))
        .addOption(new Option('--txFilePath <file>', 'unsigned transaction will be stored'))
        .action((contractName, policy, options) => {
            mainProcessor([contractName, policy], options, upgrade);
        });

    // Add 2nd level commands to 1st level command
    program.addCommand(deployCmd);
    program.addCommand(upgradeCmd);

    program.parse();
}
