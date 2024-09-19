'use strict';

require('dotenv').config();

const { addEnvOption } = require('../common');
const { governanceAddress } = require('./utils');

const { Option } = require('commander');

const addAmplifierOptions = (program, options) => {
    addEnvOption(program);

    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'));
    program.addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'));

    if (options.contractOptions) {
        addContractOptions(program);
    }

    if (options.storeOptions) {
        addStoreOptions(program);
    }

    if (options.storeProposalOptions) {
        addStoreProposalOptions(program);
    }

    if (options.instantiateOptions) {
        addInstantiateOptions(program);
    }

    if (options.instantiate2Options) {
        addInstantiate2Options(program);
    }

    if (options.instantiateProposalOptions) {
        addInstantiateProposalOptions(program);
    }

    if (options.executeProposalOptions) {
        addExecuteProposalOptions(program);
    }

    if (options.paramChangeProposalOptions) {
        addParamChangeProposalOptions(program);
    }

    if (options.migrateOptions) {
        addMigrateOptions(program);
    }

    if (options.proposalOptions) {
        addProposalOptions(program);
    }

    if (options.fetchCodeId) {
        program.addOption(new Option('--fetchCodeId', 'fetch code id from the chain by comparing to the uploaded code hash'));
    }

    if (options.runAs) {
        program.addOption(
            new Option('-r, --runAs <runAs>', 'the address that will execute the message. Defaults to governance address').default(
                governanceAddress,
            ),
        );
    }
};

const addContractOptions = (program) => {
    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').default('none').env('CHAINS'));
};

const addStoreOptions = (program) => {
    program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').env('ARTIFACT_PATH'));
};

const addStoreProposalOptions = (program) => {
    program.addOption(new Option('--source <source>', "a valid HTTPS URI to the contract's source code"));
    program.addOption(
        new Option('--builder <builder>', 'a valid docker image name with tag, such as "cosmwasm/workspace-optimizer:0.16.0'),
    );
    program.addOption(
        new Option('-i, --instantiateAddresses <instantiateAddresses>', 'comma separated list of addresses allowed to instantiate'),
    );
};

const addInstantiateOptions = (program) => {
    program.addOption(
        new Option('--admin <address>', 'when instantiating contract, set an admin address. Defaults to governance module account').default(
            governanceAddress,
        ),
    );
    program.addOption(new Option('-l, --label <label>', 'contract instance label'));
};

const addInstantiate2Options = (program) => {
    program.addOption(new Option('--instantiate2', 'use instantiate2 for constant address deployment'));
    program.addOption(new Option('-s, --salt <salt>', 'salt for instantiate2. defaults to contract name').env('SALT'));
};

const addInstantiateProposalOptions = (program) => {
    program.addOption(new Option('--predictOnly', 'output the predicted changes only').env('PREDICT_ONLY'));
};

const addExecuteProposalOptions = (program) => {
    program.addOption(new Option('--msg <msg>', 'json encoded execute message').makeOptionMandatory(true));
};

const addParamChangeProposalOptions = (program) => {
    program.addOption(new Option('--changes <changes>', 'parameter changes'));
};

const addMigrateOptions = (program) => {
    program.addOption(new Option('--msg <msg>', 'json encoded migration message').default('{}'));
};

const addProposalOptions = (program) => {
    program.addOption(new Option('-t, --title <title>', 'title of proposal').makeOptionMandatory(true));
    program.addOption(new Option('-d, --description <description>', 'description of proposal').makeOptionMandatory(true));
    program.addOption(new Option('--deposit <deposit>', 'the proposal deposit').makeOptionMandatory(true));
};

module.exports = {
    addAmplifierOptions,
};
