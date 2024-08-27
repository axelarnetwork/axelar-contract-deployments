'use strict';

require('dotenv').config();

const fs = require('fs');
const { Option } = require('commander');

// A path to the chain configuration files
const CHAIN_CONFIG_PATH = `${__dirname}/../axelar-chains-config/info`;

// A list of available chain environments which are the names of the files in the CHAIN_CONFIG_PATH
const CHAIN_ENVIRONMENTS = fs.readdirSync(CHAIN_CONFIG_PATH).map((chainName) => chainName.split('.')[0]);

const addBaseOptions = (program, options = {}) => {
    program.addOption(
        new Option('-e, --env <env>', 'environment').choices(CHAIN_ENVIRONMENTS).default('testnet').makeOptionMandatory(true).env('ENV'),
    );
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(new Option('--parallel', 'run script parallely wrt chains'));
    program.addOption(new Option('--saveChainSeparately', 'save chain info separately'));
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

module.exports = {
    CHAIN_CONFIG_PATH,
    CHAIN_ENVIRONMENTS,
    addBaseOptions,
};
