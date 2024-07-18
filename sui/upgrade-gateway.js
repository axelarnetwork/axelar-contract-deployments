const { Command, Option } = require('commander');
const { TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { bcs } = require('@mysten/sui.js/bcs');
const { fromB64, toB64 } = require('@mysten/bcs');
const { saveConfig, printInfo, validateParameters, writeJSON } = require('../evm/utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet, broadcast } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

async function processCommand(chain, options) {
    const [keypair, client] = getWallet(chain, options);
    printInfo('Wallet address', keypair.toSuiAddress());

    const { offline, policy, sender, txFilePath } = options;

    if (!chain.contracts.axelar_gateway) {
        chain.contracts.axelar_gateway = {};
    }

    const contractsConfig = chain.contracts;
    const gatewayConfig = contractsConfig.axelar_gateway;

    const builder = new TxBuilder(client);

    const { modules, dependencies, digest } = await builder.getContractBuild('axelar_gateway');

    const upgradeCap = options.upgradeCap || gatewayConfig.objects?.UpgradeCap;
    const digestHash = options.digest ? fromB64(options.digest) : digest;

    validateParameters({ isNonEmptyString: { upgradeCap, policy }, isNonEmptyStringArray: { modules, dependencies } });

    const tx = builder.tx;
    const cap = tx.object(upgradeCap);

    const ticket = tx.moveCall({
        target: `0x2::package::authorize_upgrade`,
        arguments: [cap, tx.pure(policy), tx.pure(bcs.vector(bcs.u8()).serialize(digestHash).toBytes())],
    });

    const receipt = tx.upgrade({
        modules,
        dependencies,
        packageId: gatewayConfig.address,
        ticket,
    });

    tx.moveCall({
        target: `0x2::package::commit_upgrade`,
        arguments: [cap, receipt],
    });

    if (offline) {
        sender ? tx.setSender(sender) : tx.setSender(keypair.toSuiAddress());
        const txBytes = await tx.build({ client });
        validateParameters({ isNonEmptyString: { txFilePath } });
        const txB64Bytes = toB64(txBytes);

        writeJSON({ status: 'PENDING', bytes: txB64Bytes }, txFilePath);
        printInfo(`The unsigned transaction is`, txB64Bytes);
    } else {
        const result = await broadcast(client, keypair, tx);

        const packageId = (result.objectChanges?.filter((a) => a.type === 'published') ?? [])[0].packageId;
        gatewayConfig.address = packageId;
        printInfo('Transaction result', JSON.stringify(result, null, 2));
        printInfo(`Gateway upgraded`, packageId);
    }
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);

    await processor(config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('upgrade-gateway').description('Deploy/Upgrade the Sui Gateway');

    addBaseOptions(program);

    program.addOption(new Option('--upgradeCap <upgradeCap>', 'gateway UpgradeCap id'));
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
