'use strict';

require('../common/cli-utils');

const xrpl = require('xrpl');
const { Option } = require('commander');
const { addEnvOption } = require('../common/cli-utils');

const addWalletOptions = (program, _options = {}) => {
    program.addOption(
        new Option('--walletKeyType <walletKeyType>', 'wallet key type')
            .makeOptionMandatory(true)
            .choices([xrpl.ECDSA.secp256k1])
            .default(xrpl.ECDSA.secp256k1)
            .env('WALLET_KEY_TYPE'),
    );

    return program;
};

const addSkipPromptOption = (program, _options = {}) => {
    program.addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'));
    return program;
};

const addBaseOptions = (program, _options = {}) => {
    addEnvOption(program);
    addWalletOptions(program);

    program.addOption(new Option('-n, --chainName <chainName>', 'chain to run the script over').makeOptionMandatory(true).env('CHAIN'));

    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));

    program.addOption(
        new Option('--privateKeyType <privateKeyType>', 'private key type')
            .makeOptionMandatory(true)
            .choices(['seed', 'hex'])
            .default('seed')
            .env('PRIVATE_KEY_TYPE'),
    );

    return program;
};

module.exports = {
    addBaseOptions,
    addWalletOptions,
    addSkipPromptOption,
};
