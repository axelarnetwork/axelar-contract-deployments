'use strict';

require('../common/cli-utils');

const { isNumber, addEnvOption } = require('../common');
const { addStoreOptions } = require('../common/cli-utils');
const { CONTRACT_SCOPE_CHAIN, CONTRACT_SCOPE_GLOBAL, CONTRACTS, governanceAddress, getContractCodePath } = require('./utils');

const { Option, InvalidArgumentError } = require('commander');

const addAmplifierOptions = (program, options) => {
    addEnvOption(program);
    addAxelarNodeOption(program);

    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'));
    program.addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'));

    if (options.contractOptions) {
        addContractOptions(program);
    }

    if (options.storeOptions) {
        addStoreOptions(program);
        program.hook('preAction', async (thisCommand) => {
            const opts = thisCommand.opts();
            const contractName = opts.contractName;
            const contractNames = contractName;

            const contractCodePaths = {};
            for (const name of contractNames) {
                contractCodePaths[name] = await getContractCodePath(opts, name);
            }

            Object.assign(opts, {
                contractCodePath: contractNames.length === 1 ? contractCodePaths[contractNames[0]] : undefined,
                contractCodePaths,
            });
        });
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

    if (options.codeId) {
        program.addOption(
            new Option('--codeId <codeId>', 'the code id of the contract previously uploaded').argParser((value) => {
                const parsedValue = parseInt(value, 10);

                if (!isNumber(parsedValue)) {
                    throw new InvalidArgumentError('Not a valid number.');
                }

                return parsedValue;
            }),
        );
    }

    if (options.fetchCodeId) {
        program.addOption(new Option('--fetchCodeId', 'fetch code id from the chain by comparing to the uploaded code hash'));
    }

    if (options.runAs) {
        program.addOption(new Option('-r, --runAs <runAs>', 'the address that will execute the message. Defaults to governance address'));
    }
};

const addChainNameOption = (program) => {
    program.addOption(new Option('-n, --chainName <chainName>', 'chain name').env('CHAIN').argParser((value) => value.toLowerCase()));
};

const addAmplifierQueryOptions = (program) => {
    addEnvOption(program);
    addAxelarNodeOption(program);
    addChainNameOption(program);
};

const addAxelarNodeOption = (program) => {
    program.addOption(new Option('-u, --node <axelarNode>', 'axelar node url').env('AXELAR_NODE'));
};

const addAmplifierQueryContractOptions = (program) => {
    addEnvOption(program);
    addAxelarNodeOption(program);

    addContractOptions(program);
};

const addContractOptions = (program) => {
    program.addOption(new Option('-c, --contractName <contractName...>', 'contract name(s)').makeOptionMandatory(true));
    addChainNameOption(program);
    program.hook('preAction', (command) => {
        const chainName = command.opts().chainName;
        const contractName = command.opts().contractName;
        const contractNames = contractName;

        contractNames.forEach((name) => {
            if (!CONTRACTS[name]) {
                throw new Error(`contract ${name} is not supported`);
            }
            if (!CONTRACTS[name].makeInstantiateMsg) {
                throw new Error(`makeInstantiateMsg function for contract ${name} is not defined`);
            }
            const scope = CONTRACTS[name].scope;
            if (!scope) {
                throw new Error(`scope of contract ${name} is not defined`);
            }
            if (scope === CONTRACT_SCOPE_CHAIN && !chainName) {
                throw new Error(`${name} requires chainName option`);
            }
            if (scope === CONTRACT_SCOPE_GLOBAL && chainName) {
                throw new Error(`${name} does not support chainName option`);
            }
        });
    });
};

const addStoreProposalOptions = (program) => {
    program.addOption(new Option('--source <source>', "a valid HTTPS URI to the contract's source code"));
    program.addOption(
        new Option('--builder <builder>', 'a valid docker image name with tag, such as "cosmwasm/workspace-optimizer:0.16.0'),
    );
    program.addOption(
        new Option(
            '-i, --instantiateAddresses <instantiateAddresses>',
            'comma separated list of addresses allowed to instantiate',
        ).argParser((addresses) => addresses.split(',').map((address) => address.trim())),
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
    program.addOption(
        new Option(
            '--msg <msg...>',
            'json encoded execute message(s). Can be specified multiple times for multiple messages in one proposal',
        ).makeOptionMandatory(true),
    );
};

const addParamChangeProposalOptions = (program) => {
    program.addOption(new Option('--changes <changes>', 'parameter changes'));
};

const addMigrateOptions = (program) => {
    program.addOption(
        new Option('--msg <msg>', "json encoded migration message. Use '{}' to denote an empty migration message").makeOptionMandatory(
            true,
        ),
    );
};

const addProposalOptions = (program) => {
    program.addOption(new Option('-t, --title <title>', 'title of proposal').makeOptionMandatory(true));
    program.addOption(new Option('-d, --description <description>', 'description of proposal').makeOptionMandatory(true));
    program.addOption(new Option('--deposit <deposit>', 'the proposal deposit'));
};

module.exports = {
    addAmplifierOptions,
    addAmplifierQueryOptions,
    addAmplifierQueryContractOptions,
    addChainNameOption,
};
