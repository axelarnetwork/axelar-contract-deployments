'use strict';

const { Option } = require('commander');
const { addEnvOption } = require('../common/cli-utils');

const addBaseOptions = (program, _options = {}) => {
    addEnvOption(program);

    program.addOption(
        new Option('-n, --chainName <chainName>', 'chain to run the script over').makeOptionMandatory(true).env('CHAIN'),
    );

    program.addOption(new Option('-s, --seed <seed>', 'seed used to derive wallet keypair').makeOptionMandatory(true).env('SEED'));

    return program;
};

module.exports = {
    ...require('../common/cli-utils'),
    addBaseOptions,
};
