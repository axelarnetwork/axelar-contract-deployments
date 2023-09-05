'use strict';

require('dotenv').config();

const { Command, Option } = require('commander');
const { loadConfig, saveConfig } = require('./utils');

async function processCommand(options, chain, _) {
    const { contractName } = options;

    const contracts = chain.contracts;

    if (contracts[contractName]) {
        delete contracts[contractName];
    }
}

async function main(options) {
    const config = loadConfig(options.env);

    let chains = options.chainNames.split(',').map((str) => str.trim());

    if (options.chainNames === 'all') {
        chains = Object.keys(config.chains);
    }

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chain} is not defined in the info file`);
        }
    }

    for (const chain of chains) {
        await processCommand(options, config.chains[chain.toLowerCase()], config);
        saveConfig(config, options.env);
    }
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
