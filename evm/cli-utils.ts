import { Command, Option } from 'commander';

import { BaseOptions, addBaseOptions, addEnvOption, addOptionsToCommands, addStoreOptions } from '../common/cli-utils';

interface EvmOptions extends BaseOptions {
    artifactPath?: boolean;
    contractName?: boolean;
    deployMethod?: string;
    salt?: boolean;
    skipExisting?: boolean;
    upgrade?: boolean;
    predictOnly?: boolean;
}

const addEvmOptions = (program: Command, options: EvmOptions = {}): Command => {
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

const addTopUpOptions = (program: Command): void => {
    program.addOption(new Option('-t, --target <target>', 'target balance for each account').makeOptionMandatory(true));
    program.addOption(
        new Option('--threshold <threshold>', 'top up accounts only if the balance is below this threshold').makeOptionMandatory(true),
    );
    program.addOption(new Option('-u, --units', 'amounts are set in smallest unit'));
    program.addOption(
        new Option(
            '--addresses-to-derive <addresses-to-derive>',
            'number of addresses to derive from mnemonic. Derived addresses will be added to the list of addresses to fund set by using --addresses option',
        ).env('DERIVE_ACCOUNTS'),
    );
    program.addOption(
        new Option('--addresses <addresses>', 'comma separated list of addresses to top up')
            .default([])
            .argParser((addresses: string): string[] => addresses.split(',').map((address) => address.trim())),
    );
    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').env('MNEMONIC'));
};

const addGovernanceOptions = (program: Command): Command => {
    program.addOption(new Option('--governance', 'Submit this change via interchain governance'));
    program.addOption(
        new Option(
            '--governanceEta <eta>',
            'Governance ETA (YYYY-MM-DDTHH:mm:ss UTC) or 0 for immediate scheduling (subject to min timelock)',
        ).default('0'),
    );
    program.addOption(new Option('--file <file>', 'File to write Axelar proposal JSON to'));

    return program;
};

module.exports = {
    addEnvOption,
    addBaseOptions,
    addOptionsToCommands,
    addStoreOptions,
    addEvmOptions,
    addTopUpOptions,
    addGovernanceOptions,
};

export type { BaseOptions, EvmOptions };
