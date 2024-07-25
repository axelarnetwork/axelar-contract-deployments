const { saveConfig, printInfo, validateParameters, writeJSON } = require('../evm/utils');
const { Command, Option } = require('commander');
const { updateMoveToml, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, hexlify, toUtf8Bytes, keccak256 },
    constants: { HashZero },
} = ethers;
const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { loadSuiConfig, getAmplifierSigners, deployPackage } = require('./utils');
const { bytes32Struct, signersStruct } = require('./types-utils');
const { bcs } = require('@mysten/sui/bcs');
const { Transaction } = require('@mysten/sui/transactions');
const { upgradePackage } = require('./deploy-utils');
const { toB64 } = require('@mysten/sui/utils');

// Add more contracts here to support more modules deployment
const contractMap = {
    GasService: {
        packageName: 'gas_service',
    },
    AxelarGateway: {
        packageName: 'axelar_gateway',
    },
};

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

async function deploy(contractName, config, chain, options) {
    const contract = contractMap[contractName];
    const packageName = options.packageName || contract.packageName;

    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts[contractName]) {
        chain.contracts[contractName] = {};
    }

    const { packageId, publishTxn } = await deployPackage(packageName, client, keypair, options);

    const contractConfig = chain.contracts[contractName];
    contractConfig.address = packageId;
    contractConfig.objects = {};

    switch (contractName) {
        case 'GasService': {
            const contractObject = publishTxn.objectChanges.find(
                (change) => change.objectType === `${packageId}::${packageName}::${contractName}`,
            );
            contractConfig.objects[contractName] = contractObject.objectId;
            const gasCollectorCapObject = publishTxn.objectChanges.find(
                (change) => change.objectType === `${packageId}::${packageName}::GasCollectorCap`,
            );
            contractConfig.objects.GasCollectorCap = gasCollectorCapObject.objectId;
            break;
        }

        case 'AxelarGateway': {
            const { minimumRotationDelay, domainSeparator } = options;
            const signers = await getSigners(keypair, config, chain, options);
            const operator = options.operator || keypair.toSuiAddress();
            const { previousSigners } = options;

            validateParameters({ isNonEmptyString: { previousSigners } });

            const creatorCap = publishTxn.objectChanges.find((change) => change.objectType === `${packageId}::gateway::CreatorCap`);

            const relayerDiscovery = publishTxn.objectChanges.find(
                (change) => change.objectType === `${packageId}::discovery::RelayerDiscovery`,
            );

            const upgradeCap = publishTxn.objectChanges.find((change) => change.objectType === '0x2::package::UpgradeCap').objectId;

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
                    tx.object(creatorCap.objectId),
                    tx.pure.address(operator),
                    separator,
                    tx.pure.u64(minimumRotationDelay),
                    tx.pure.u64(options.previousSigners),
                    tx.pure(bcs.vector(bcs.u8()).serialize(encodedSigners).toBytes()),
                    tx.object('0x6'),
                ],
            });
            const result = await broadcast(client, keypair, tx);

            const gateway = result.objectChanges.find((change) => change.objectType === `${packageId}::gateway::Gateway`);

            contractConfig.address = packageId;
            contractConfig.objects = {
                gateway: gateway.objectId,
                relayerDiscovery: relayerDiscovery.objectId,
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

async function upgrade(contractName, policy, config, chain, options) {
    const [keypair, client] = getWallet(chain, options);
    const { packageDependencies } = options;
    const contract = contractMap[contractName];
    const packageName = options.packageName || contract.packageName;
    options.policy = policy;

    printInfo('Wallet address', keypair.toSuiAddress());

    if (!chain.contracts[packageName]) {
        chain.contracts[packageName] = {};
    }

    const contractsConfig = chain.contracts;
    const packageConfig = contractsConfig?.[contractName];

    validateParameters({ isNonEmptyString: { packageName } });

    if (packageDependencies) {
        for (const dependencies of packageDependencies) {
            const packageId = contractsConfig[dependencies]?.address;
            updateMoveToml(dependencies, packageId);
        }
    }

    const builder = new TxBuilder(client);
    await upgradePackage(client, keypair, packageName, packageConfig, builder, options);
}

async function mainProcessor(args, options, processor) {
    const config = loadSuiConfig(options.env);
    await processor(...args, config, config.sui, options);
    saveConfig(config, options.env);

    if (options.offline) {
        const { txFilePath } = options;
        validateParameters({ isNonEmptyString: { txFilePath } });

        const txB64Bytes = toB64(options.txBytes);

        writeJSON({ message: options.offlineMessage, status: 'PENDING', unsignedTx: txB64Bytes }, txFilePath);
        printInfo(`The unsigned transaction is`, txB64Bytes);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-contract').description('Deploy/Upgrade packages');

    const deployCMD = program
        .name('deploy')
        .description('Deploy SUI modules')
        .command('deploy <contractName>')
        .addOption(new Option('--packageName <packageName>', 'Package name to deploy'))
        .addOption(new Option('--signers <signers>', 'JSON with the initial signer set').env('SIGNERS'))
        .addOption(new Option('--operator <operator>', 'operator for the gateway (defaults to the deployer address)').env('OPERATOR'))
        .addOption(new Option('--minimumRotationDelay <minimumRotationDelay>', 'minium delay for signer rotations (in ms)').default(0))
        .addOption(new Option('--domainSeparator <domainSeparator>', 'domain separator').default(HashZero))
        .addOption(new Option('--nonce <nonce>', 'nonce for the signer (defaults to HashZero)'))
        .addOption(new Option('--previousSigners <previousSigners>', 'number of previous signers to retain').default('15'))
        .action((contractName, options) => {
            mainProcessor([contractName], options, deploy);
        });

    const upgradeCMD = program
        .name('upgrade')
        .description('Upgrade SUI modules')
        .command('upgrade <contractName> <policy>')
        .addOption(new Option('--packageName <packageName>', 'package name to upgrade'))
        .addOption(new Option('--sender <sender>', 'transaction sender'))
        .addOption(new Option('--digest <digest>', 'digest hash for upgrade'))
        .addOption(new Option('--offline', 'store tx block for sign'))
        .addOption(new Option('--txFilePath <file>', 'unsigned transaction will be stored'))
        .action((contractName, policy, options) => {
            mainProcessor([contractName, policy], options, upgrade);
        });

    addBaseOptions(deployCMD);
    addBaseOptions(upgradeCMD);

    program.parse();
}
