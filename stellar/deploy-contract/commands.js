'use strict';

const { Command, Option } = require('commander');
const { getContractCodePath, SUPPORTED_CONTRACTS, sanitizeMigrationData } = require('../utils');
const { addStoreOptions } = require('../../common/cli-utils');
const { mainProcessor, upgrade, upload, deploy } = require('./processors');

require('../cli-utils');

const CONTRACT_CONFIG = {
    AxelarGateway: {
        deployOptions: [
            ['--nonce <nonce>', 'optional nonce for the signer set'],
            ['--domain-separator <domainSeparator>', 'domain separator (keccak256 hash or "offline")', { default: 'offline' }],
            ['--previous-signers-retention <previousSignersRetention>', 'previous signer retention', { default: 15, parser: Number }],
            ['--minimum-rotation-delay <miniumRotationDelay>', 'minimum rotation delay', { default: 0, parser: Number }],
        ],
        upgradeOptions: [['--migration-data <migrationData>', 'migration data', { default: null, defaultDescription: '()' }]],
    },
    AxelarExample: {
        deployOptions: [
            ['--use-dummy-its-address', 'use dummy its address for AxelarExample contract to test a GMP call', { default: false }],
        ],
    },
    AxelarOperators: {
        upgradeOptions: [['--migration-data <migrationData>', 'migration data', { default: null, defaultDescription: '()' }]],
    },
    InterchainToken: {
        args: [
            ['<name>', 'token name (e.g., "Test Token")'],
            ['<symbol>', 'token symbol (e.g., "TEST")'],
            ['<decimals>', 'token decimals (e.g., 7)', parseInt],
        ],
        optionKeys: ['name', 'symbol', 'decimals'],
    },
    InterchainTokenService: {
        args: [
            ['<interchain-token-version>', 'version for InterchainToken contract'],
            ['<token-manager-version>', 'version for TokenManager contract'],
        ],
        optionKeys: ['interchainTokenVersion', 'tokenManagerVersion'],
        upgradeOptions: [['--migration-data <migrationData>', 'migration data', { default: null, defaultDescription: '()' }]],
    },
};

const createOption = ([flag, description, config = {}]) => {
    const option = new Option(flag, description);
    if (config.default !== undefined) option.default(config.default, config.defaultDescription);
    if (config.parser) option.argParser(config.parser);
    return option;
};

const addOptionsToCommand = (command, optionType) => {
    addStoreOptions(command);

    const contractName = command.name();
    const options = CONTRACT_CONFIG[contractName]?.[optionType];

    if (options) {
        options.forEach((optionConfig) => {
            command.addOption(createOption(optionConfig));
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

const addArgumentsToCommand = (command, contractName) => {
    const config = CONTRACT_CONFIG[contractName];
    if (!config?.args) return;

    config.args.forEach(([name, description, parser]) => {
        command.argument(name, description, parser);
    });
};

const createActionHandler = (contractName, processor) => {
    const config = CONTRACT_CONFIG[contractName];

    if (!config?.optionKeys) {
        return (options) => mainProcessor(options, processor, contractName);
    }

    return (...args) => {
        const options = args.pop();
        config.optionKeys.forEach((key, index) => {
            options[key] = args[index];
        });
        mainProcessor(options, processor, contractName);
    };
};

const getDeployContractCommands = () => {
    return Array.from(SUPPORTED_CONTRACTS).map((contractName) => {
        const command = new Command(contractName).description(`Deploy ${contractName} contract`);

        addArgumentsToCommand(command, contractName);
        addOptionsToCommand(command, 'deployOptions');

        command.hook('preAction', preActionHook(contractName));
        command.action(createActionHandler(contractName, deploy));

        return command;
    });
};

const getUpgradeContractCommands = () => {
    return Array.from(SUPPORTED_CONTRACTS).map((contractName) => {
        const command = new Command(contractName).description(`Upgrade ${contractName} contract`);

        addOptionsToCommand(command, 'upgradeOptions');

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

        addOptionsToCommand(command, null);

        command.hook('preAction', preActionHook(contractName));
        command.action((options) => mainProcessor(options, upload, contractName));

        return command;
    });
};

module.exports = {
    getDeployContractCommands,
    getUpgradeContractCommands,
    getUploadContractCommands,
};
