'use strict';

const { Command, Option } = require('commander');
const { mainProcessor } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

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

if (require.main === module) {
    const program = new Command();

    program.name('remove-info').description('Remove info about contract from the info file.');

    addBaseOptions(program, { ignorePrivateKey: true });

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
