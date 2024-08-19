const { bcs } = require('@mysten/bcs');
const { fromB64 } = require('@mysten/bcs');
const { printInfo, validateParameters } = require('../../common/utils');
const { getObjectIdsByObjectTypes, suiPackageAddress } = require('./utils');
const UPGRADE_POLICIES = {
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

async function upgradePackage(client, keypair, packageToUpgrade, contractConfig, builder, options) {
    const { packageDir, packageName } = packageToUpgrade;
    const { modules, dependencies, digest } = await builder.getContractBuild(packageDir);
    const { offline } = options;
    const sender = options.sender || keypair.toSuiAddress();
    const upgradeCap = contractConfig.objects?.UpgradeCap;
    const digestHash = options.digest ? fromB64(options.digest) : digest;
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

    tx.setSender(sender);
    const txBytes = await tx.build({ client });

    if (offline) {
        options.txBytes = txBytes;
        options.offlineMessage = `Transaction to upgrade ${packageDir}`;
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
        contractConfig.address = packageId;
        const [upgradeCap] = getObjectIdsByObjectTypes(result, [`${suiPackageAddress}::package::UpgradeCap`]);
        contractConfig.objects.UpgradeCap = upgradeCap;

        printInfo('Transaction Digest', JSON.stringify(result.digest, null, 2));
        printInfo(`${packageName} Upgraded Address`, packageId);
    }
}

module.exports = {
    UPGRADE_POLICIES,
    upgradePackage,
};
