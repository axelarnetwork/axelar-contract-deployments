const { bcs } = require('@mysten/bcs');
const { fromBase64 } = require('@mysten/bcs');
const { Transaction } = require('@mysten/sui/transactions');
const { printInfo, validateParameters } = require('../../common/utils');
const { copyMovePackage, updateMoveToml } = require('@axelar-network/axelar-cgp-sui');
const { getObjectIdsByObjectTypes, suiPackageAddress, moveDir, saveGeneratedTx } = require('./utils');
const { broadcast } = require('./sign-utils');

const UPGRADE_POLICIES = {
    immutable: 'make_immutable',
    code_upgrade: 'only_additive_upgrades',
    dependency_upgrade: 'only_dep_upgrades',
};

function getUpgradePolicyId(policy) {
    switch (policy) {
        case 'any_upgrade':
            return 0;
        case 'code_upgrade':
            return 128;
        case 'dep_upgrade':
            return 192;
        default:
            throw new Error(`Unknown upgrade policy: ${policy}. Supported policies: any_upgrade, code_upgrade, dep_upgrade`);
    }
}

function restrictUpgradePolicy(tx, policy, upgradeCap) {
    const upgradeType = UPGRADE_POLICIES[policy];

    if (upgradeType) {
        tx.moveCall({
            target: `${suiPackageAddress}::package::${upgradeType}`,
            arguments: [tx.object(upgradeCap)],
        });
    }

    return tx;
}

function broadcastRestrictedUpgradePolicy(client, keypair, upgradeCap, options) {
    const upgradeType = UPGRADE_POLICIES[options.policy];

    if (!upgradeType) {
        return;
    }

    if (!upgradeCap) {
        throw new Error(`Cannot find upgrade cap to restrict upgrade policy`);
    }

    return broadcast(
        client,
        keypair,
        restrictUpgradePolicy(new Transaction(), options.policy, upgradeCap),
        `Restricted Upgrade Policy (${options.policy})`,
        options,
    );
}

async function upgradePackage(client, keypair, packageToUpgrade, contractConfig, builder, options) {
    copyMovePackage(packageToUpgrade.packageDir, null, moveDir);
    const { packageDir, packageName } = packageToUpgrade;
    const { modules, dependencies, digest } = await builder.getContractBuild(packageDir, moveDir);
    const { offline } = options;
    const upgradeCap = contractConfig.objects?.UpgradeCap;
    const digestHash = options.digest ? fromBase64(options.digest) : digest;
    const policy = getUpgradePolicyId(options.policy);

    validateParameters({ isNonEmptyString: { upgradeCap }, isNonEmptyStringArray: { modules, dependencies } });

    const tx = builder.tx;
    const cap = tx.object(upgradeCap);

    const ticket = tx.moveCall({
        target: `${suiPackageAddress}::package::authorize_upgrade`,
        arguments: [cap, tx.pure.u8(policy), tx.pure(bcs.vector(bcs.u8()).serialize(digestHash).toBytes())],
    });

    const receipt = tx.upgrade({
        modules,
        dependencies,
        package: contractConfig.address,
        ticket,
    });

    tx.moveCall({
        target: `${suiPackageAddress}::package::commit_upgrade`,
        arguments: [cap, receipt],
    });

    const sender = options.sender || keypair.toSuiAddress();
    tx.setSender(sender);

    if (offline) {
        await saveGeneratedTx(tx, `Transaction to upgrade ${packageDir}`, client, options);
    } else {
        const txBytes = await tx.build({ client });
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

        printInfo('Transaction Digest', JSON.stringify(result.digest, null, 2));

        const publishedObject = result.objectChanges.find((change) => change.type === 'published');
        const packageId = publishedObject.packageId;
        const versionNumber = parseInt(publishedObject.version) - 1;
        const [upgradeCap] = getObjectIdsByObjectTypes(result, [`${suiPackageAddress}::package::UpgradeCap`]);

        contractConfig.address = packageId;
        contractConfig.versions[versionNumber] = publishedObject.packageId;
        contractConfig.objects.UpgradeCap = upgradeCap;

        printInfo(`${packageName} Upgraded Address`, packageId);
        updateMoveToml(packageToUpgrade.packageDir, packageId, moveDir);

        return { upgraded: result, packageId };
    }
}

module.exports = {
    UPGRADE_POLICIES,
    upgradePackage,
    restrictUpgradePolicy,
    broadcastRestrictedUpgradePolicy,
};
