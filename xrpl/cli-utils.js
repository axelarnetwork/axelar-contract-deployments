'use strict';

const { Option } = require('commander');
const { addEnvOption } = require('../common/cli-utils');

const addBaseOptions = (program, _options = {}) => {
    addEnvOption(program);

    program.addOption(
        new Option('-n, --chainName <chainName>', 'chain to run the script over').makeOptionMandatory(true).env('CHAIN'),
    );

    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));

    program.addOption(new Option('--privateKeyType <privateKeyType>', 'private key type')
        .makeOptionMandatory(true)
        .choices(['seed'])
        .default('seed')
        .env('PRIVATE_KEY_TYPE'),
    );

    return program;
};

module.exports = {
    ...require('../common/cli-utils'),
    addBaseOptions,
};
