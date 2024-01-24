'use strict';

require('dotenv').config();

const { Option } = require('commander');

const addBaseOptions = (program, options = {}) => {
    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(new Option('--gasOptions <gasOptions>', 'gas options cli override'));

    if (!options.ignoreChainNames) {
        program.addOption(
            new Option('-n, --chainNames <chainNames>', 'chains to run the script over').makeOptionMandatory(true).env('CHAINS'),
        );
        program.addOption(new Option('--skipChains <skipChains>', 'chains to skip over'));
        program.addOption(
            new Option(
                '--startFromChain <startFromChain>',
                'start from a specific chain onwards in the config, useful when a cmd fails for an intermediate chain',
            ),
        );
    }

    if (!options.ignorePrivateKey) {
        program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    }

    if (options.address) {
        program.addOption(new Option('-a, --address <address>', 'override address'));
    }

    return program;
};

const addExtendedOptions = (program, options = {}) => {
    addBaseOptions(program, options);

    program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));

    if (options.artifactPath) {
        program.addOption(new Option('--artifactPath <artifactPath>', 'artifact path'));
    }

    if (options.contractName) {
        program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
    }

    if (options.salt) {
        program.addOption(new Option('-s, --salt <salt>', 'salt to use for create2 deployment').env('SALT'));
    }

    if (options.skipExisting) {
        program.addOption(new Option('-x, --skipExisting', 'skip existing if contract was already deployed on chain').env('SKIP_EXISTING'));
    }

    if (options.upgrade) {
        program.addOption(new Option('-u, --upgrade', 'upgrade a deployed contract').env('UPGRADE'));
    }

    if (options.predictOnly) {
        program.addOption(new Option('--predictOnly', 'output the predicted changes only').env('PREDICT_ONLY'));
    }

    return program;
};

module.exports = {
    addBaseOptions,
    addExtendedOptions,
};
