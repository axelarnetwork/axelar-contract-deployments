const { saveConfig, prompt, printInfo } = require('../common/utils');
const { Command } = require('commander');
const { deployPackage, getBcsBytesByObjectId } = require('./utils');
const { singletonStruct } = require('./types-utils');
const { Transaction } = require('@mysten/sui/transactions');
const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');

// Parse bcs bytes from singleton object to get channel id
async function getChannelId(client, singletonObjectId) {
    const bcsBytes = await getBcsBytesByObjectId(client, singletonObjectId);
    const data = singletonStruct.parse(bcsBytes);
    return '0x' + data.channel.id;
}

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

    const published = await deployPackage('test', client, keypair);

    const singleton = published.publishTxn.objectChanges.find((change) => change.objectType === `${published.packageId}::test::Singleton`);

    const tx = new Transaction();

    tx.moveCall({
        target: `${published.packageId}::test::register_transaction`,
        arguments: [tx.object(relayerDiscovery), tx.object(singleton.objectId)],
    });

    await broadcast(client, keypair, tx);

    const channelId = await getChannelId(client, singleton.objectId);

    chain.contracts.test.address = published.packageId;
    chain.contracts.test.objects = { singleton: singleton.objectId, channelId };

    printInfo('Test package deployed', JSON.stringify(chain.contracts.test, null, 2));
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);

    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-test').description('Deploys/publishes the test module');

    addBaseOptions(program);

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
