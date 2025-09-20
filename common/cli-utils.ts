'use strict';

import { Command, Option } from 'commander';
import * as dotenv from 'dotenv';
import * as fs from 'fs';
import * as path from 'path';

dotenv.config();

// Resolve the path relative to the package root, accounting for both source and dist directories
const resolveFromRoot = (relativePath: string) => {
    // Check if we're in dist directory
    if (__dirname.includes('dist')) {
        return path.join(__dirname, '../../', relativePath);
    }
    // We're in the source directory
    return path.join(__dirname, '../', relativePath);
};

const CHAIN_CONFIG_PATH = resolveFromRoot('axelar-chains-config/info');
const CHAIN_ENVIRONMENTS = fs.readdirSync(CHAIN_CONFIG_PATH).map((chainName: string) => chainName.split('.')[0]);

interface BaseOptions {
    ignoreChainNames?: boolean;
    ignorePrivateKey?: boolean;
    address?: boolean;
}

const addEnvOption = (program: Command, defaultValue?: string): void => {
    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(CHAIN_ENVIRONMENTS)
            .default(defaultValue || 'testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
};

const addBaseOptions = (program: Command, options: BaseOptions = {}): Command => {
    addEnvOption(program);

    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(new Option('--parallel', 'run script in parallel wrt chains'));
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

const addOptionsToCommands = <T>(program: Command, optionMethod: (command: Command, options: T) => void, options: T): void => {
    if (program.commands.length > 0) {
        program.commands.forEach((command) => {
            optionMethod(command, options);
        });
    }
};

const addStoreOptions = (program: Command): void => {
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

export { addEnvOption, addBaseOptions, addOptionsToCommands, addStoreOptions };
export type { BaseOptions };

module.exports = {
    addEnvOption,
    addBaseOptions,
    addOptionsToCommands,
    addStoreOptions,
};
