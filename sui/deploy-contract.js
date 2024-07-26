const { saveConfig, printInfo } = require('../evm/utils');
const { Command, Argument, Option } = require('commander');
const { addBaseOptions } = require('./cli-utils');
const { Transaction } = require('@mysten/sui/transactions');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { singletonStruct } = require('./types-utils');
const { loadSuiConfig, findPublishedObject, deployPackage, getBcsBytesByObjectId } = require('./utils');

// Add more contracts here to support more modules deployment
const contractMap = {
    GasService: {
        packageName: 'gas_service',
    },
    Test: {
        packageName: 'test',
    },
};

const postDeploy = {
    GasService: postDeployGasService,
    Test: postDeployTest,
};

// Parse bcs bytes from singleton object to get channel id
async function getChannelId(client, singletonObjectId) {
    const bcsBytes = await getBcsBytesByObjectId(client, singletonObjectId);
    const data = singletonStruct.parse(bcsBytes);
    return '0x' + data.channel.id;
}

async function postDeployGasService(published, config, chain, options) {
    chain.contracts.GasService.objects.GasCollectorCap = findPublishedObject(
        published,
        contractMap.GasService.packageName,
        'GasCollectorCap',
    ).objectId;

    chain.contracts.GasService.objects.GasService = findPublishedObject(
        published,
        contractMap.GasService.packageName,
        'GasService',
    ).objectId;
}

async function postDeployTest(published, config, chain, options) {
    const [keypair, client] = getWallet(chain, options);

    const singleton = published.publishTxn.objectChanges.find((change) => change.objectType === `${published.packageId}::test::Singleton`);
    const relayerDiscovery = config.sui.contracts.axelar_gateway?.objects?.relayerDiscovery;

    const tx = new Transaction();
    tx.moveCall({
        target: `${published.packageId}::test::register_transaction`,
        arguments: [tx.object(relayerDiscovery), tx.object(singleton.objectId)],
    });

    const registerTx = await broadcast(client, keypair, tx);

    printInfo('Register transaction', registerTx.digest);
}

async function processCommand(contractName, config, chain, options) {
    const contract = contractMap[contractName];
    const packageName = options.packageName || contract.packageName;

    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts[contractName]) {
        chain.contracts[contractName] = {};
    }

    const published = await deployPackage(packageName, client, keypair);
    const packageId = published.packageId;

    const contractConfig = chain.contracts[contractName];
    contractConfig.address = packageId;

    printInfo(`${contractName} deployed`, JSON.stringify(contractConfig, null, 2));

    // Submitting additional setup transaction or saving additional objects to the chain config here
    await postDeploy[contractName]?.(published, config, chain, options);
}

async function mainProcessor(contractName, options, processor) {
    const config = loadSuiConfig(options.env);
    await processor(contractName, config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-contract')
        .addOption(new Option('--packageName <packageName>', 'Package name to deploy'))
        .addArgument(new Argument('<contractName>', 'Contract name to deploy').choices(Object.keys(contractMap)))
        .description('Deploy SUI modules');

    addBaseOptions(program);

    program.action((contractName, options) => {
        mainProcessor(contractName, options, processCommand);
    });

    program.parse();
}
