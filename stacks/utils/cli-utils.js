'use strict';

const { Option } = require('commander');
const { addEnvOption } = require('../../common');

const addBaseOptions = (program, options = {}) => {
    addEnvOption(program);
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(
        new Option('-n, --chainName <chainName>', 'chain name')
            .env('CHAIN')
            .default('stacks')
            .argParser((value) => value.toLowerCase()),
    );

    if (!options.ignorePrivateKey) {
        program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').env('STACKS_MNEMONIC'));
        program.addOption(new Option('-p, --privateKey <privateKey>', 'privateKey').env('STACKS_PRIVATE_KEY'));
    }

    return program;
};

module.exports = {
    ...require('../../common/cli-utils'),
    addBaseOptions,
};
