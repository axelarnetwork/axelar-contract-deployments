const { saveConfig, printInfo } = require('../evm/utils');
const { Command, Argument } = require('commander');
const { publishPackage, updateMoveToml } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

// Add more contracts here to support more modules deployment
const contractMap = {
    gas_service: {
        packageName: 'gas_service',
        contractName: 'GasService',
        displayName: 'Gas Service',
        chainConfigKey: 'axelar_gas_service',
    },
};

async function processCommand(contractName, config, chain, options) {
    const contract = contractMap[contractName];

    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts[contract.chainConfigKey]) {
        chain.contracts[contract.chainConfigKey] = {};
    }

    const published = await publishPackage(contract.packageName, client, keypair);
    const packageId = published.packageId;

    updateMoveToml(contract.packageName, packageId);

    const contractObject = published.publishTxn.objectChanges.find(
        (change) => change.objectType === `${packageId}::${contract.packageName}::${contract.contractName}`,
    );
    const gasCollectorCapObject = published.publishTxn.objectChanges.find(
        (change) => change.objectType === `${packageId}::${contract.packageName}::GasCollectorCap`,
    );

    const contractConfig = chain.contracts[contract.chainConfigKey];
    contractConfig.address = packageId;
    contractConfig.objects = {
        [contractName]: contractObject.objectId,
    };

    switch (contractName) {
        case contractMap.gas_service.packageName:
            contractConfig.objects.gas_collector_cap = gasCollectorCapObject.objectId;
            break;
    }

    printInfo(`${contract.displayName} deployed`, JSON.stringify(contractConfig, null, 2));
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
        .addArgument(new Argument('<contractName>', 'Contract name to deploy').choices(Object.keys(contractMap)))
        .description('Deploy SUI modules');

    addBaseOptions(program);

    program.action((contractName, options) => {
        mainProcessor(contractName, options, processCommand);
    });

    program.parse();
}
