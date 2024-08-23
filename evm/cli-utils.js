const { addBaseOptions, ...exportedCliUtils } = require('../common/cli-utils');

const addExtendedOptions = (program, options = {}) => {
    addBaseOptions(program, options);

    program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));

    if (options.artifactPath) {
        program.addOption(new Option('--artifactPath <artifactPath>', 'artifact path'));
    }

    if (options.contractName) {
        program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
    }

    if (options.deployMethod) {
        program.addOption(
            new Option('-m, --deployMethod <deployMethod>', 'deployment method')
                .choices(['create', 'create2', 'create3'])
                .default(options.deployMethod),
        );
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
    ...exportedCliUtils,
    addBaseOptions,
    addExtendedOptions,
};
