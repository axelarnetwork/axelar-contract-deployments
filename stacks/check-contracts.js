'use strict';

const { saveConfig, loadConfig, printInfo, printWarn, getChainConfig, kebabToPascal, printError } = require('../common/utils');
const { addBaseOptions } = require('./utils');
const { Command } = require('commander');
const { fetchAbi } = require('@stacks/transactions/dist/fetch');

const CONTRACTS_TO_CHECK = [
    'clarity-stacks',
    'gas-storage',
    'traits',
    'gas-impl',
    'gateway-storage',
    'gas-service',
    'gateway-impl',
    'gateway',
    'governance',
    'interchain-token-service-storage',
    'interchain-token-factory',
    'interchain-token-service',
    'interchain-token-factory-impl',
    'verify-onchain',
    'interchain-token-service-impl',
    'native-interchain-token',
];

async function processCommand(config, chain) {
    printInfo(`Checking contracts...`);

    let hasError = false;
    for (const contract of CONTRACTS_TO_CHECK) {
        if (!chain.contracts[kebabToPascal(contract)]?.address) {
            printError(`Contract ${contract} does not exist in the config`);
            hasError = true;
        } else {
            const [contractAddress, contractName] = chain.contracts[kebabToPascal(contract)].address.split('.');

            try {
                await fetchAbi({
                    network: chain.networkType,
                    contractAddress,
                    contractName,
                });
            } catch (e) {
                printError(`Contract ${contract} not found on chain`);
                hasError = true;
            }
        }
    }

    if (!hasError) {
        printInfo(`Finished checking contracts, everything was OK!`);
    } else {
        printError('There has been an error while checking one or more contracts!');
    }
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(config, chain, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('check-contracts')
        .description('Check that all the contracts have been deployed')
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program, { ignorePrivateKey: true });

    program.parse();
}
