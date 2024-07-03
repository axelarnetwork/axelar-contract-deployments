const { Command, Option } = require('commander');
const { publishPackage, updateMoveToml, getContractBuild } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { fromB64, toB64 } = require('@mysten/bcs');
const { saveConfig, printInfo, validateParameters, prompt, writeJSON } = require('../evm/utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

async function processCommand(chain, options) {
    const [keypair, client] = getWallet(chain, options);
    const { offline, upgrade, sender, txFilePath } = options;

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts.its) {
        chain.contracts.its = {};
    }

    const contractConfig = chain.contracts;
    const itsContractConfig = contractConfig?.its;

    const abiPackageId = contractConfig.abi?.address;
    const gatewayPackageId = contractConfig.axelar_gateway?.address;
    const governancePackageId = contractConfig.governance?.address;

    validateParameters({ isKeccak256Hash: { abiPackageId, gatewayPackageId, governancePackageId } });

    updateMoveToml('abi', abiPackageId);
    updateMoveToml('axelar_gateway', gatewayPackageId);
    updateMoveToml('governance', governancePackageId);

    if (upgrade) {
        const { modules, dependencies, digest } = await getContractBuild('its');
        const { policy } = options;

        const upgradeCap = itsContractConfig.objects?.upgradeCap;
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
            packageId: itsContractConfig.address,
            ticket,
        });

        tx.moveCall({
            target: `0x2::package::commit_upgrade`,
            arguments: [cap, receipt],
        });

        if (!offline) {
            const result = await broadcast(client, keypair, tx);

            const packageId = (result.objectChanges?.filter((a) => a.type === 'published') ?? [])[0].packageId;
            itsContractConfig.address = packageId;
            printInfo('Transaction result', JSON.stringify(result, null, 2));
            printInfo('ITS upgraded', packageId);
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
    } else {
        if (!offline) {
            if (prompt(`Proceed with deployment on ${chain.name}?`, options.yes)) {
                return;
            }

            const published = await publishPackage('its', client, keypair);
            const packageId = published.packageId;
            itsContractConfig.address = packageId;
            const ITS = published.publishTxn.objectChanges.find((change) => change.objectType === `${packageId}::its::ITS`);
            const upgradeCap = published.publishTxn.objectChanges.find((change) => change.objectType === `0x2::package::UpgradeCap`);
            itsContractConfig.objects = {
                its: ITS.objectId,
                upgradeCap: upgradeCap.objectId,
            };

            printInfo('ITS deployed', JSON.stringify(itsContractConfig, null, 2));
        } else {
            validateParameters({ isNonEmptyString: { txFilePath } });

            const txBytes = await publishPackage('its', client, keypair, options);
            writeJSON({ status: 'DEPLOY PENDING', bytes: txBytes }, txFilePath);
            printInfo(`The unsigned transaction is`, txBytes);
        }
    }
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);

    await processor(config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-its').description('Deploy/Upgrade the Sui ITS');

    addBaseOptions(program);

    program.addOption(new Option('--upgrade', 'deploy or upgrade ITS'));
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
