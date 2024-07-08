const { Command, Option } = require('commander');
const { publishPackage, updateMoveToml, getContractBuild } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { fromB64, toB64 } = require('@mysten/bcs');
const { saveConfig, printInfo, validateParameters, prompt, writeJSON } = require('../evm/utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet, broadcast } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

async function upgradePackage(client, keypair, packageName, packageConfig, options) {
    const { modules, dependencies, digest } = await getContractBuild(packageName);
    const { policy, offline, txFilePath, sender } = options;

    const upgradeCap = packageConfig.objects?.UpgradeCap;
    let digestHash;

    if (options.digest) {
        digestHash = fromB64(options.digest);
    } else {
        digestHash = digest;
    }

    validateParameters({ isNonEmptyString: { upgradeCap, policy }, isNonEmptyStringArray: { modules, dependencies } });

    const tx = new TransactionBlock();
    const cap = tx.object(upgradeCap);

    const ticket = tx.moveCall({
        target: `0x2::package::authorize_upgrade`,
        arguments: [cap, tx.pure(policy), tx.pure(bcs.vector(bcs.u8()).serialize(digestHash).toBytes())],
    });

    const receipt = tx.upgrade({
        modules,
        dependencies,
        packageId: packageConfig.address,
        ticket,
    });

    tx.moveCall({
        target: `0x2::package::commit_upgrade`,
        arguments: [cap, receipt],
    });

    if (!offline) {
        const result = await broadcast(client, keypair, tx);

        const packageId = (result.objectChanges?.filter((a) => a.type === 'published') ?? [])[0].packageId;
        packageConfig.address = packageId;
        printInfo('Transaction result', JSON.stringify(result, null, 2));
        printInfo(`${packageName} upgraded`, packageId);
    } else {
        validateParameters({ isNonEmptyString: { txFilePath } });

        if (!sender) {
            tx.setSender(keypair.toSuiAddress());
        } else {
            tx.setSender(sender);
        }

        const txBytes = await tx.build({ client });
        writeJSON({ status: 'UPGRADE PENDING', bytes: toB64(txBytes) }, txFilePath);
        printInfo(`The unsigned transaction is`, toB64(txBytes));
    }
}

async function deployPackage(chain, client, keypair, packageName, packageConfig, options) {
    const { offline, txFilePath } = options;

    if (!offline) {
        if (prompt(`Proceed with deployment on ${chain.name}?`, options.yes)) {
            return;
        }

        const published = await publishPackage(packageName, client, keypair);
        const packageId = published.packageId;
        packageConfig.address = packageId;
        const objectChanges = published.publishTxn.objectChanges.filter((object) => object.type === 'created');
        packageConfig.objects = {};

        for (const object of objectChanges) {
            const firstIndex = object.objectType.indexOf('::');
            const typeIndex = object.objectType.indexOf('::', firstIndex + 1);
            const objectName = object.objectType.slice(typeIndex + 2);

            if (objectName) {
                packageConfig.objects[objectName] = object.objectId;
            }
        }

        printInfo(`${packageName} deployed`, JSON.stringify(packageConfig, null, 2));
    } else {
        validateParameters({ isNonEmptyString: { txFilePath } });

        const txBytes = await publishPackage(packageName, client, keypair, options);
        writeJSON({ status: 'DEPLOY PENDING', bytes: txBytes }, txFilePath);
        printInfo(`The unsigned transaction is`, txBytes);
    }
}

async function processCommand(chain, options) {
    const [keypair, client] = getWallet(chain, options);
    const { upgrade, packageName, packageDependencies } = options;

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

    if (upgrade) {
        await upgradePackage(client, keypair, packageName, packageConfig, options);
    } else {
        await deployPackage(chain, client, keypair, packageName, packageConfig, options);
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
