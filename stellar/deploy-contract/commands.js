'use strict';

const { Command, Option, Argument } = require('commander');
const { getContractCodePath, SUPPORTED_CONTRACTS, sanitizeMigrationData, addBaseOptions } = require('../utils');
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
    InterchainTokenService: () => [
        new Option('--interchain-token-version <interchainTokenVersion>', 'version for InterchainToken contract').makeOptionMandatory(true),
        new Option('--token-manager-version <tokenManagerVersion>', 'version for TokenManager contract').makeOptionMandatory(true),
    ],
    InterchainToken: () => [
        new Argument('<name>', 'token name (e.g., "Test Token")'),
        new Argument('<symbol>', 'token symbol (e.g., "TEST")'),
        new Argument('<decimals>', 'token decimals (e.g., 7)').argParser(Number),
    ],
};

const CONTRACT_UPGRADE_OPTIONS = {
    AxelarGateway: () => [new Option('--migration-data <migrationData>', 'migration data').default(null, '()')],
    AxelarOperators: () => [new Option('--migration-data <migrationData>', 'migration data').default(null, '()')],
    InterchainTokenService: () => [new Option('--migration-data <migrationData>', 'migration data').default(null, '()')],
};

const CONTRACT_UPLOAD_OPTIONS = {};

const addContractOptions = (command, commandType) => {
    addBaseOptions(command);
    addStoreOptions(command);

    const contractName = command.name();
    const contractOptions = {
        deploy: CONTRACT_DEPLOY_OPTIONS,
        upgrade: CONTRACT_UPGRADE_OPTIONS,
        upload: CONTRACT_UPLOAD_OPTIONS,
    }[commandType];

    if (contractOptions && contractOptions[contractName]) {
        const items = contractOptions[contractName]();
        items.forEach((item) => {
            if (item instanceof Option) {
                command.addOption(item);
            } else if (item instanceof Argument) {
                command.addArgument(item);
            } else {
                throw new Error(
                    `Invalid item type in contract ${commandType} options for ${contractName}: expected Option or Argument, got ${item?.constructor?.name || typeof item}`,
                );
            }
        });
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

const createContractCommands = (commandType, processor, preProcessor = null) => {
    const descriptions = {
        deploy: 'Deploy',
        upgrade: 'Upgrade',
        upload: 'Upload',
    };

    return Array.from(SUPPORTED_CONTRACTS).map((contractName) => {
        const command = new Command(contractName).description(`${descriptions[commandType]} ${contractName} contract`);

        addContractOptions(command, commandType);
        command.hook('preAction', preActionHook(contractName));

        command.action((...actionArgs) => {
            const cmd = actionArgs.pop();
            const options = cmd.opts();
            const args = actionArgs;

            if (preProcessor) {
                preProcessor(options, contractName);
            }

            mainProcessor(processor, contractName, args, options);
        });

        return command;
    });
};

const getDeployContractCommands = () => createContractCommands('deploy', deploy);

const getUpgradeContractCommands = () =>
    createContractCommands('upgrade', upgrade, (options, contractName) => {
        options.migrationData = sanitizeMigrationData(options.migrationData, options.version, contractName);
    });

const getUploadContractCommands = () => createContractCommands('upload', upload);

module.exports = {
    getDeployContractCommands,
    getUpgradeContractCommands,
    getUploadContractCommands,
};
