'use strict';

const { Option } = require('commander');
const { addEnvOption } = require('../../common');

const addBaseOptions = (program, options = {}) => {
    addEnvOption(program);
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(new Option('--gasOptions <gasOptions>', 'gas options cli override'));
    program.addOption(
        new Option('-n, --chainName <chainName>', 'chain name')
            .env('CHAIN')
            .default('stacks')
            .argParser((value) => value.toLowerCase()),
    );

    if (!options.ignorePrivateKey) {
        program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic')
          .makeOptionMandatory(true)
          .env('STACKS_MNEMONIC')
        );
    }

    return program;
};

module.exports = {
    ...require('../../common/cli-utils'),
    addBaseOptions,
};
