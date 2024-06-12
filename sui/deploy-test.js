const { saveConfig, prompt, printInfo } = require('../evm/utils');
const { Command, Option } = require('commander');
const { publishPackage, updateMoveToml } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { ethers } = require('hardhat');
const {
    constants: { HashZero },
} = ethers;
const { loadSuiConfig } = require('./utils');

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');

async function processCommand(config, chain, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts.test) {
        chain.contracts.test = {};
    }

    const relayerDiscovery = config.sui.contracts.axelar_gateway?.objects?.relayerDiscovery;

    if (!relayerDiscovery) {
        throw new Error('Relayer discovery object not found');
    }

    if (prompt(`Proceed with deployment on ${chain.name}?`, options.yes)) {
        return;
    }

    const published = await publishPackage('test', client, keypair);
    updateMoveToml('test', published.packageId);

    const singleton = published.publishTxn.objectChanges.find((change) => change.objectType === `${published.packageId}::test::Singleton`);

    const tx = new TransactionBlock();

    tx.moveCall({
        target: `${published.packageId}::test::register_transaction`,
        arguments: [tx.object(relayerDiscovery), tx.object(singleton.objectId)],
    });

    await broadcast(client, keypair, tx);

    chain.contracts.test.address = published.packageId;
    chain.contracts.test.objects = { singleton: singleton.objectId };

    printInfo('Test package deployed', JSON.stringify(chain.contracts.test, null, 2));
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);

    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-gateway').description('Deploys/publishes the Sui gateway');

    addBaseOptions(program);

    program.addOption(new Option('--signers <signers>', 'JSON with the initial signer set').env('SIGNERS'));
    program.addOption(new Option('--operator <operator>', 'operator for the gateway (defaults to the deployer address)'));
    program.addOption(
        new Option('--minimumRotationDelay <minimumRotationDelay>', 'minium delay for signer rotations (in ms)').default(
            24 * 60 * 60 * 1000,
        ),
    ); // 1 day (in ms)
    program.addOption(new Option('--domainSeparator <domainSeparator>', 'domain separator').default(HashZero));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
