'use strict';

require('dotenv').config();

const { Command, Option } = require('commander');
const { mainProcessor, addEnvironmentOptions } = require('./utils');

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

addEnvironmentOptions(program);

program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));

program.action((options) => {
    main(options);
});

program.parse();
