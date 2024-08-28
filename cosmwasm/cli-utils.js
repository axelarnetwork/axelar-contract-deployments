'use strict';

require('dotenv').config();

const { addEnvOption } = require('../common');
const { governanceAddress } = require('./utils');

const { Option } = require('commander');

const addAmplifierOptions = (program) => {
    addEnvOption(program);

    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'));
    program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').env('ARTIFACT_PATH'));
    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').default('none').env('CHAINS'));

    program.addOption(new Option('-s, --salt <salt>', 'salt for instantiate2. defaults to contract name').env('SALT'));
    program.addOption(
        new Option('--admin <address>', 'when instantiating contract, set an admin address. Defaults to governance module account').default(
            governanceAddress,
        ),
    );
    program.addOption(new Option('--instantiate2', 'use instantiate2 for constant address deployment'));
    program.addOption(new Option('--fetchCodeId', 'fetch code id from the chain by comparing to the uploaded code hash'));
    program.addOption(new Option('-l, --label <label>', 'contract instance label'));

    program.addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'));
};

module.exports = {
    addAmplifierOptions,
};
