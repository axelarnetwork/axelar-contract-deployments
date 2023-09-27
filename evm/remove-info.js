'use strict';

require('dotenv').config();

const { Command, Option } = require('commander');
const { mainProcessor } = require('./utils');

async function processCommand(options, chain, _) {
    const { contractName } = options;

    const contracts = chain.contracts;

    if (contracts[contractName]) {
        delete contracts[contractName];
    }
}

async function main(options) {
    await mainProcessor(options, processCommand, true);
}

const program = new Command();

program.name('remove-info').description('Remove info about contract from the info file.');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));

program.action((options) => {
    main(options);
});

program.parse();
