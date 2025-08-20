'use strict';

require('dotenv').config();

const fs = require('fs');
const { Option } = require('commander');

// A path to the chain configuration files
const CHAIN_CONFIG_PATH = `${__dirname}/../axelar-chains-config/info`;

// A list of available chain environments which are the names of the files in the CHAIN_CONFIG_PATH
const CHAIN_ENVIRONMENTS = fs.readdirSync(CHAIN_CONFIG_PATH).map((chainName) => chainName.split('.')[0]);

const addEnvOption = (program, defaultValue) => {
    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(CHAIN_ENVIRONMENTS)
            .default(defaultValue || 'testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
};

const addBaseOptions = (program, options = {}) => {
    addEnvOption(program);

    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(new Option('--parallel', 'run script parallely wrt chains'));
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

// `optionMethod` is a method such as `addBaseOptions`
// `options` is an option object for optionMethod
const addOptionsToCommands = (program, optionMethod, options) => {
    if (program.commands.length > 0) {
        program.commands.forEach((command) => {
            optionMethod(command, options);
        });
    }
};

const addStoreOptions = (program) => {
    program.addOption(
        new Option(
            '-a, --artifact-dir <artifactDir>',
            'Path to the contract artifact directory to upload (required if --version is not used)',
        ).env('ARTIFACT_DIR'),
    );

    program.addOption(
        new Option(
            '-v, --version <contractVersion>',
            'Specify a released version (X.Y.Z) or a commit hash to upload (required if --artifact-dir is not used)',
        ).env('CONTRACT_VERSION'),
    );

    program.hook('preAction', async (thisCommand) => {
        const opts = thisCommand.opts();

        if (!opts.artifactDir && !opts.version) {
            throw new Error('Either --artifact-dir or --version is required');
        }
    });
};

module.exports = {
    addEnvOption,
    addBaseOptions,
    addOptionsToCommands,
    addStoreOptions,
};
