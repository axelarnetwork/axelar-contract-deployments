const { Command, Option } = require('commander');
const { TxBuilder, updateMoveToml } = require('@axelar-network/axelar-cgp-sui');
const { bcs } = require('@mysten/sui/bcs');
const { fromB64, toB64 } = require('@mysten/bcs');
const { saveConfig, printInfo, validateParameters, prompt, writeJSON } = require('../evm/utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

async function upgradePackage(client, keypair, packageName, packageConfig, builder, options) {
    const { modules, dependencies, digest } = await builder.getContractBuild(packageName);
    const { policy, offline } = options;
    const sender = options.sender || keypair.toSuiAddress();
    const suiPackageId = '0x2';

    const upgradeCap = packageConfig.objects?.UpgradeCap;
    const digestHash = options.digest ? fromB64(options.digest) : digest;

    validateParameters({ isNonEmptyString: { upgradeCap, policy }, isNonEmptyStringArray: { modules, dependencies } });

    const tx = builder.tx;
    const cap = tx.object(upgradeCap);

    const ticket = tx.moveCall({
        target: `${suiPackageId}::package::authorize_upgrade`,
        arguments: [cap, tx.pure(policy), tx.pure(bcs.vector(bcs.u8()).serialize(digestHash).toBytes())],
    });

    const receipt = tx.upgrade({
        modules,
        dependencies,
        packageId: packageConfig.address,
        ticket,
    });

    tx.moveCall({
        target: `${suiPackageId}::package::commit_upgrade`,
        arguments: [cap, receipt],
    });

    tx.setSender(sender);
    const txBytes = await tx.build({ client });

    if (offline) {
        options.txBytes = txBytes;
    } else {
        const signature = (await keypair.signTransaction(txBytes)).signature;
        const result = await client.executeTransactionBlock({
            transactionBlock: txBytes,
            signature,
            options: {
                showEffects: true,
                showObjectChanges: true,
                showEvents: true,
            },
        });

        const packageId = (result.objectChanges?.filter((a) => a.type === 'published') ?? [])[0].packageId;
        packageConfig.address = packageId;
        printInfo('Transaction result', JSON.stringify(result, null, 2));
        printInfo(`${packageName} upgraded`, packageId);
    }
}

async function deployPackage(chain, client, keypair, packageName, packageConfig, builder, options) {
    const { offline, sender } = options;

    const address = sender || keypair.toSuiAddress();
    await builder.publishPackageAndTransferCap(packageName, address);
    const tx = builder.tx;
    tx.setSender(address);
    const txBytes = await tx.build({ client });

    if (offline) {
        options.txBytes = txBytes;
    } else {
        if (prompt(`Proceed with deployment on ${chain.name}?`, options.yes)) {
            return;
        }

        const signature = (await keypair.signTransaction(txBytes)).signature;
        const publishTxn = await client.executeTransactionBlock({
            transactionBlock: txBytes,
            signature,
            options: {
                showEffects: true,
                showObjectChanges: true,
                showEvents: true,
            },
        });

        packageConfig.address = (publishTxn.objectChanges?.find((a) => a.type === 'published') ?? []).packageId;
        const objectChanges = publishTxn.objectChanges.filter((object) => object.type === 'created');
        packageConfig.objects = {};

        for (const object of objectChanges) {
            const array = object.objectType.split('::');
            const objectName = array[array.length - 1];

            if (objectName) {
                packageConfig.objects[objectName] = object.objectId;
            }
        }

        printInfo(`${packageName} deployed`, JSON.stringify(packageConfig, null, 2));
    }
}

async function processCommand(chain, options) {
    const [keypair, client] = getWallet(chain, options);
    const { upgrade, packageName, packageDependencies, offline, txFilePath } = options;

    printInfo('Wallet address', keypair.toSuiAddress());

    if (!chain.contracts[packageName]) {
        chain.contracts[packageName] = {};
    }

    const contractsConfig = chain.contracts;
    const packageConfig = contractsConfig?.[packageName];

    validateParameters({ isNonEmptyString: { packageName } });

    if (packageDependencies) {
        for (const dependencies of packageDependencies) {
            const packageId = contractsConfig[dependencies]?.address;
            updateMoveToml(dependencies, packageId);
        }
    }

    const builder = new TxBuilder(client);

    if (upgrade) {
        await upgradePackage(client, keypair, packageName, packageConfig, builder, options);
    } else {
        await deployPackage(chain, client, keypair, packageName, packageConfig, builder, options);
    }

    if (offline) {
        validateParameters({ isNonEmptyString: { txFilePath } });

        const txB64Bytes = toB64(options.txBytes);

        writeJSON({ status: 'PENDING', bytes: txB64Bytes }, txFilePath);
        printInfo(`The unsigned transaction is`, txB64Bytes);
    }
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);

    await processor(config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-upgrade').description('Deploy/Upgrade the Sui package');

    addBaseOptions(program);

    program.addOption(new Option('--packageName <packageName>', 'package name to deploy/upgrade'));
    program.addOption(new Option('--packageDependencies [packageDependencies...]', 'array of package dependencies'));
    program.addOption(new Option('--upgrade', 'deploy or upgrade'));
    program.addOption(new Option('--policy <policy>', 'new policy to upgrade'));
    program.addOption(new Option('--sender <sender>', 'transaction sender'));
    program.addOption(new Option('--digest <digest>', 'digest hash for upgrade'));
    program.addOption(new Option('--offline', 'store tx block for sign'));
    program.addOption(new Option('--txFilePath <file>', 'unsigned transaction will be stored'));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
