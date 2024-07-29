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
const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { loadSuiConfig, getAmplifierSigners, deployPackage, getObjectIdsByObjectTypes } = require('./utils');
const { bytes32Struct, signersStruct } = require('./types-utils');
const { upgradePackage } = require('./deploy-utils');
const { suiPackageAddress, suiClockAddress } = require('./utils');

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

    printInfo(`${contractName} deployed`, JSON.stringify(contractConfig, null, 2));
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
    const program = new Command();

    program.name('deploy-contract').description('Deploy/Upgrade packages');

    const deployCmd = program
        .name('deploy')
        .description('Deploy a Sui package')
        .command('deploy <contractName>')
        .addOption(new Option('--signers <signers>', 'JSON with the initial signer set').env('SIGNERS'))
        .addOption(new Option('--operator <operator>', 'operator for the gateway (defaults to the deployer address)').env('OPERATOR'))
        .addOption(
            new Option('--minimumRotationDelay <minimumRotationDelay>', 'minium delay for signer rotations (in second)')
                .default(24 * 60 * 60)
                .parseArg((val) => parseInt(val) * 1000),
        )
        .addOption(new Option('--domainSeparator <domainSeparator>', 'domain separator'))
        .addOption(new Option('--nonce <nonce>', 'nonce for the signer (defaults to HashZero)'))
        .addOption(new Option('--previousSigners <previousSigners>', 'number of previous signers to retain').default('15'))
        .addOption(
            new Option('--policy <policy>', 'upgrade policy for upgrade cap: For example, use "any_upgrade" to allow all types of upgrades')
                .choices(['any_upgrade', 'code_upgrade', 'dep_upgrade'])
                .default('any_upgrade'),
        )
        .action((contractName, options) => {
            mainProcessor([contractName], options, deploy);
        });

    const upgradeCmd = program
        .name('upgrade')
        .description('Upgrade a Sui package')
        .command('upgrade <contractName> <policy>')
        .addOption(new Option('--sender <sender>', 'transaction sender'))
        .addOption(new Option('--digest <digest>', 'digest hash for upgrade'))
        .addOption(new Option('--offline', 'store tx block for sign'))
        .addOption(new Option('--txFilePath <file>', 'unsigned transaction will be stored'))
        .action((contractName, policy, options) => {
            mainProcessor([contractName, policy], options, upgrade);
        });

    addBaseOptions(deployCmd);
    addBaseOptions(upgradeCmd);

    program.parse();
}
