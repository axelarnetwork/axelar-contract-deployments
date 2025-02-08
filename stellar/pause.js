const { Contract } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { getWallet, broadcast, addBaseOptions } = require('./utils');
const { loadConfig, printInfo } = require('../evm/utils');
const { getChainConfig } = require('../common');
require('./cli-utils');

async function processCommand(options, _, chain) {
    const wallet = await getWallet(chain, options);

    const contract = new Contract(options.address || chain.contracts?.pausable_contract?.address);

    let operation;

    switch (options.action) {
        case 'paused': {
            operation = contract.call('paused');
            break;
        }

        case 'pause': {
            operation = contract.call('pause');
            break;
        }

        case 'unpause': {
            operation = contract.call('unpause');
            break;
        }

        default: {
            throw new Error(`Unknown action: ${options.action}`);
        }
    }

    const returnValue = await broadcast(operation, wallet, chain, `${options.action} performed`, options);

    if (returnValue.value()) {
        printInfo('Return value', returnValue.value());
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('pausable').description('Pausable contract management');

    addBaseOptions(program, { address: true });
    program.addOption(
        new Option('--action <action>', 'pausable contract action').choices(['paused', 'pause', 'unpause']).makeOptionMandatory(true),
    );

    program.action((options) => {
        const config = loadConfig(options.env);
        processCommand(options, config, getChainConfig(config, options.chainName));
    });

    program.parse();
}
