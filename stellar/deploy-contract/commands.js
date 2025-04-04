'use strict';

const { Command, Option } = require('commander');
const { getContractCodePath, SUPPORTED_CONTRACTS, sanitizeMigrationData } = require('../utils');
const { addStoreOptions } = require('../../common/cli-utils');
const { mainProcessor, upgrade, upload, deploy } = require('./processors');

require('../cli-utils');

const CONTRACT_DEPLOY_OPTIONS = {
    AxelarGateway: () => [
        new Option('--nonce <nonce>', 'optional nonce for the signer set'),
        new Option('--domain-separator <domainSeparator>', 'domain separator (keccak256 hash or "offline")').default('offline'),
        new Option('--previous-signers-retention <previousSignersRetention>', 'previous signer retention').default(15).argParser(Number),
        new Option('--minimum-rotation-delay <miniumRotationDelay>', 'minimum rotation delay').default(0).argParser(Number),
    ],
    AxelarExample: () => [
        new Option('--use-dummy-its-address', 'use dummy its address for AxelarExample contract to test a GMP call').default(false),
    ],
};

const CONTRACT_UPGRADE_OPTIONS = {
    AxelarGateway: () => [new Option('--migration-data <migrationData>', 'migration data').default(null, '()')],
    AxelarOperators: () => [new Option('--migration-data <migrationData>', 'migration data').default(null, '()')],
    InterchainTokenService: () => [new Option('--migration-data <migrationData>', 'migration data').default(null, '()')],
};

const CONTRACT_UPLOAD_OPTIONS = {};

const addDeployOptions = (command) => {
    const contractName = command.name();
    const contractDeployOptions = CONTRACT_DEPLOY_OPTIONS[contractName];

    if (contractDeployOptions) {
        const options = contractDeployOptions();
        // Add the options to the program
        options.forEach((option) => command.addOption(option));
    }

    return command;
};

const addUpgradeOptions = (command) => {
    const contractName = command.name();
    const contractUpgradeOptions = CONTRACT_UPGRADE_OPTIONS[contractName];

    if (contractUpgradeOptions) {
        const options = contractUpgradeOptions();
        options.forEach((option) => command.addOption(option));
    }

    return command;
};

const addUploadOptions = (command) => {
    const contractName = command.name();
    const contractUploadOptions = CONTRACT_UPLOAD_OPTIONS[contractName];

    if (contractUploadOptions) {
        const options = contractUploadOptions();
        options.forEach((option) => command.addOption(option));
    }

    return command;
};

function preActionHook(contractName) {
    return async (thisCommand) => {
        const opts = thisCommand.opts();

        const contractCodePath = await getContractCodePath(opts, contractName);
        Object.assign(opts, { contractCodePath });
    };
}

const getDeployContractCommands = () => {
    return Array.from(SUPPORTED_CONTRACTS).map((contractName) => {
        const command = new Command(contractName).description(`Deploy ${contractName} contract`);

        addStoreOptions(command);
        addDeployOptions(command);

        command.hook('preAction', preActionHook(contractName));
        command.action((options) => {
            mainProcessor(options, deploy, contractName);
        });

        return command;
    });
};

const getUpgradeContractCommands = () => {
    return Array.from(SUPPORTED_CONTRACTS).map((contractName) => {
        const command = new Command(contractName).description(`Upgrade ${contractName} contract`).addHelpText(
            'after',
            `
Examples:
  # using Vec<Address> as migration data:
  $ deploy-contract upgrade axelar-operators deploy --artifact-path {releasePath}/stellar_axelar_operators.optimized.wasm --version 2.1.7 --migration-data '["GDYBNA2LAWDKRSCIR4TKCB5LJCDRVUWKHLMSKUWMJ3YX3BD6DWTNT5FW"]'

  # default void migration data:
  $ deploy-contract upgrade axelar-gateway deploy --artifact-path {releasePath}/stellar_axelar_gateway.optimized.wasm --version 1.0.1

  # equivalent explicit void migration data:
  $ deploy-contract upgrade axelar-gateway deploy --artifact-path {releasePath}/stellar_axelar_gateway.optimized.wasm --version 1.0.1 --migration-data '()'
`,
        );

        addStoreOptions(command);
        addUpgradeOptions(command);

        command.hook('preAction', preActionHook(contractName));
        command.action((options) => {
            options.migrationData = sanitizeMigrationData(options.migrationData, options.version, contractName);
            mainProcessor(options, upgrade, contractName);
        });

        return command;
    });
};

const getUploadContractCommands = () => {
    return Array.from(SUPPORTED_CONTRACTS).map((contractName) => {
        const command = new Command(contractName).description(`Upload ${contractName} contract`);

        addStoreOptions(command);
        addUploadOptions(command);

        command.hook('preAction', preActionHook(contractName));
        command.action((options) => {
            mainProcessor(options, upload, contractName);
        });

        return command;
    });
};

module.exports = {
    getDeployContractCommands,
    getUpgradeContractCommands,
    getUploadContractCommands,
};
