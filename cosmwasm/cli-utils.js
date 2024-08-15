'use strict';

require('dotenv').config();

const { addExtendedOptions } = require('../common');
const { governanceAddress } = require('./utils');

const { Option } = require('commander');

const addCommonAmplifierOptions = (program, options = {}) => {
    const ops = {
        ignoreParallel: true,
        ignoreSaveChainSeparately: true,
        ignoreGasOptions: true,
        ignoreChainNames: true,
        ignorePrivateKey: true,
        ignoreVerify: true,
        contractName: true,
        salt: true,
        ...options,
    };

    addExtendedOptions(program, ops);

    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'));
    program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true).env('ARTIFACT_PATH'));
    program.addOption(new Option('--aarch64', 'aarch64').env('AARCH64').default(false));
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').default('none'));
    program.addOption(
        new Option('--admin <address>', 'when instantiating contract, set an admin address. Defaults to governance module account').default(
            governanceAddress,
        ),
    );
    program.addOption(new Option('--instantiate2', 'use instantiate2 for constant address deployment'));
    program.addOption(new Option('--fetchCodeId', 'fetch code id from the chain by comparing to the uploaded code hash'));
    program.addOption(new Option('-l, --label <label>', 'contract instance label'));
};

module.exports = {
    addCommonAmplifierOptions,
};
