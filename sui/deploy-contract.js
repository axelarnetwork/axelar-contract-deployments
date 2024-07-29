const { saveConfig, printInfo } = require('../evm/utils');
const { Command, Argument } = require('commander');
const { addBaseOptions } = require('./cli-utils');
const { Transaction } = require('@mysten/sui/transactions');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { singletonStruct } = require('./types-utils');
const { loadSuiConfig, findPublishedObject, deployPackage, getBcsBytesByObjectId, readMovePackageName } = require('./utils');

// A list of currently supported packages which are the folder names in `node_modules/@axelar-network/axelar-cgp-sui/move`
const supportedPackageDirs = ['gas_service', 'test'];

// Map supported packages to their package names and directories
const supportedPackages = supportedPackageDirs.map((dir) => ({
    packageName: readMovePackageName(dir),
    packageDir: dir,
}));

// Parse bcs bytes from singleton object to get channel id
async function getChannelId(client, singletonObjectId) {
    const bcsBytes = await getBcsBytesByObjectId(client, singletonObjectId);
    const data = singletonStruct.parse(bcsBytes);
    return '0x' + data.channel.id;
}

/** ######## Post Deployment Functions ######## **/
// Define the post deployment functions for each supported package here. These functions should be called after the package is deployed.
// Use cases include:
// 1. Update the chain config with deployed object ids
// 2. Submit additional transactions to setup the contracts.

async function postDeployGasService(published, chain) {
    chain.contracts.GasService.objects = {
        GasCollectorCap: findPublishedObject(published, 'gas_service', 'GasCollectorCap').objectId,
        GasService: findPublishedObject(published, 'gas_service', 'GasService').objectId,
    };
}

async function postDeployTest(published, config, chain, options) {
    const [keypair, client] = getWallet(chain, options);
    const relayerDiscovery = config.sui.contracts.axelar_gateway?.objects?.relayerDiscovery;

    const singleton = findPublishedObject(published, 'test', 'Singleton');
    const channelId = await getChannelId(client, singleton.objectId);
    chain.contracts.Test.objects = { singleton: singleton.objectId, channelId };

    const tx = new Transaction();
    tx.moveCall({
        target: `${published.packageId}::test::register_transaction`,
        arguments: [tx.object(relayerDiscovery), tx.object(singleton.objectId)],
    });

    const registerTx = await broadcast(client, keypair, tx);

    printInfo('Register transaction', registerTx.digest);
}

/** ######## Main Processor ######## **/

async function processCommand(supportedContract, config, chain, options) {
    const { packageDir, packageName } = supportedContract;

    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts[packageName]) {
        chain.contracts[packageName] = {};
    }

    const published = await deployPackage(packageDir, client, keypair);

    // Submitting additional setup transaction or saving additional objects to the chain config here
    switch (packageName) {
        case 'GasService':
            await postDeployGasService(published, chain);
            break;
        case 'Test':
            await postDeployTest(published, config, chain, options);
            break;
        default:
            throw new Error(`Unsupported package: ${packageName}`);
    }

    printInfo(`${packageName} deployed`, JSON.stringify(chain.contracts[packageName], null, 2));
}

async function mainProcessor(supportedContract, options, processor) {
    const config = loadSuiConfig(options.env);
    await processor(supportedContract, config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-contract')
        .addArgument(
            new Argument('<contractName>', 'Contract name to deploy')
                .choices(supportedPackages.map((p) => p.packageName))
                .argParser((packageName) => supportedPackages.find((p) => p.packageName === packageName)),
        )
        .description('Deploy SUI modules');

    addBaseOptions(program);

    program.action((supportedContract, options) => {
        mainProcessor(supportedContract, options, processCommand);
    });

    program.parse();
}
