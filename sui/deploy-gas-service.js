const { saveConfig, printInfo } = require('../evm/utils');
const { Command } = require('commander');
const { publishPackage, updateMoveToml } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

async function processCommand(config, chain, options) {
    const [keypair, client] = getWallet(chain, options);
    const packageName = 'gas_service';

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts.axelar_gas_service) {
        chain.contracts.axelar_gas_service = {};
    }

    const contractConfig = chain.contracts.axelar_gas_service;

    const published = await publishPackage(packageName, client, keypair);
    const packageId = published.packageId;

    updateMoveToml(packageName, packageId);

    const gasService = published.publishTxn.objectChanges.find(
        (change) => change.objectType === `${packageId}::${packageName}::GasService`,
    );
    contractConfig.address = packageId;
    contractConfig.objects = {
        gasService: gasService.objectId,
    };

    printInfo('\nGas Service deployed', JSON.stringify(contractConfig, null, 2));
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);
    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-gas-service').description('Deploys/publishes the Axelar Gas Service package');

    addBaseOptions(program);

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
