const { Contract } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const { getWallet, broadcast, addBaseOptions } = require('./utils');
const { loadConfig, printInfo } = require('../evm/utils');
const { getChainConfig, addOptionsToCommands } = require('../common');
require('./cli-utils');

async function paused(contract) {
    return contract.call('paused');
}

async function pause(contract) {
    return contract.call('pause');
}

async function unpause(contract) {
    return contract.call('unpause');
}

async function processCommand(processor, options, contractName) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);
    const contract = new Contract(chain.contracts?.[contractName]?.address || options.address);

    const operation = await processor(contract);

    const returnValue = await broadcast(operation, wallet, chain, `${processor.name} performed`, options);

    if (returnValue.value()) {
        printInfo('Return value', returnValue.value());
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('contract').description('Common contract operations');

    program
        .command('paused')
        .description('Check if the contract is paused')
        .argument('<contract-name>', 'contract name to deploy')
        .action((contractName, options) => {
            processCommand(paused, options, contractName);
        });

    program
        .command('pause')
        .description('Pause the contract')
        .argument('<contract-name>', 'contract name to deploy')
        .action((contractName, options) => {
            processCommand(pause, options, contractName);
        });

    program
        .command('unpause')
        .description('Unpause the contract')
        .argument('<contract-name>', 'contract name to deploy')
        .action((contractName, options) => {
            processCommand(unpause, options, contractName);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
