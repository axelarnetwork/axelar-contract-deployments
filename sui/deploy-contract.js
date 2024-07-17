const { saveConfig, printInfo } = require('../evm/utils');
const { Command, Argument, Option } = require('commander');
const { publishPackage, updateMoveToml } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo } = require('./sign-utils');
const { loadSuiConfig, findPublishedObject } = require('./utils');

// Add more contracts here to support more modules deployment
const contractMap = {
    GasService: {
        packageName: 'gas_service',
    },
};

async function processCommand(contractName, config, chain, options) {
    const contract = contractMap[contractName];
    const packageName = options.packageName || contract.packageName;

    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts[contractName]) {
        chain.contracts[contractName] = {};
    }

    const published = await publishPackage(packageName, client, keypair);
    const packageId = published.packageId;

    updateMoveToml(packageName, packageId);

    const contractObject = findPublishedObject(published, packageName, contractName);
    const gasCollectorCapObject = findPublishedObject(published, packageName, 'GasCollectorCap');

    const contractConfig = chain.contracts[contractName];
    contractConfig.address = packageId;
    contractConfig.objects = {
        [contractName]: contractObject.objectId,
    };

    switch (contractName) {
        case 'GasService':
            contractConfig.objects.GasCollectorCap = gasCollectorCapObject.objectId;
            break;
        default:
            throw new Error(`${contractName} is not supported.`);
    }

    printInfo(`${contractName} deployed`, JSON.stringify(contractConfig, null, 2));
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
