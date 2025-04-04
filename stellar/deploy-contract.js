'use strict';

const { Address, nativeToScVal, scValToNative, Operation, Contract } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { getWallet, broadcast, serializeValue, addBaseOptions, getContractCodePath, SUPPORTED_CONTRACTS, BytesToScVal } = require('./utils');
const { getDomainSeparator, getChainConfig, addOptionsToCommands } = require('../common');
const { prompt, validateParameters } = require('../common/utils');
const { addStoreOptions } = require('../common/cli-utils');
const { weightedSignersToScVal, itsCustomMigrationDataToScValV110 } = require('./type-utils');
const { ethers } = require('hardhat');
const { readFileSync } = require('fs');
const {
    utils: { arrayify, id },
} = ethers;
require('./cli-utils');

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

const CustomMigrationDataTypeToScValV110 = {
    InterchainTokenService: (migrationData) => itsCustomMigrationDataToScValV110(migrationData),
};

const VERSIONED_CUSTOM_MIGRATION_DATA_TYPES = {
    '1.1.0': CustomMigrationDataTypeToScValV110,
};

const addDeployOptions = (program) => {
    // Get the package name from the program name
    const contractName = program.name();
    // Find the corresponding options for the package
    const contractDeployOptions = CONTRACT_DEPLOY_OPTIONS[contractName];

    if (contractDeployOptions) {
        const options = contractDeployOptions();
        // Add the options to the program
        options.forEach((option) => program.addOption(option));
    }

    return program;
};

const addUpgradeOptions = (program) => {
    const contractName = program.name();
    const contractUpgradeOptions = CONTRACT_UPGRADE_OPTIONS[contractName];

    if (contractUpgradeOptions) {
        const options = contractUpgradeOptions();
        options.forEach((option) => program.addOption(option));
    }

    return program;
};

async function uploadContract(contractName, options, wallet, chain) {
    const contractCodePath = await getContractCodePath(options, contractName);
    return await uploadWasm(wallet, chain, contractCodePath);
}

async function getInitializeArgs(config, chain, contractName, wallet, options) {
    const owner = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });
    const operator = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });

    switch (contractName) {
        case 'AxelarGateway': {
            const domainSeparator = nativeToScVal(Buffer.from(arrayify(await getDomainSeparator(config, chain, options))));
            const minimumRotationDelay = nativeToScVal(options.minimumRotationDelay);
            const previousSignersRetention = nativeToScVal(options.previousSignersRetention);
            const nonce = options.nonce ? arrayify(id(options.nonce)) : Array(32).fill(0);
            const initialSigners = nativeToScVal([
                weightedSignersToScVal({
                    nonce,
                    signers: [
                        {
                            signer: wallet.publicKey(),
                            weight: 1,
                        },
                    ],
                    threshold: 1,
                }),
            ]);

            return {
                owner,
                operator,
                domainSeparator,
                minimumRotationDelay,
                previousSignersRetention,
                initialSigners,
            };
        }

        case 'InterchainTokenService': {
            const gatewayAddress = nativeToScVal(Address.fromString(chain.contracts?.AxelarGateway?.address), { type: 'address' });
            const gasServiceAddress = nativeToScVal(Address.fromString(chain.contracts?.AxelarGasService?.address), { type: 'address' });
            const itsHubAddress = nativeToScVal(config.axelar?.contracts?.InterchainTokenService?.address, { type: 'string' });
            const chainName = nativeToScVal(chain.axelarId, { type: 'string' });
            const nativeTokenAddress = nativeToScVal(Address.fromString(chain?.tokenAddress), { type: 'address' });
            const interchainTokenWasmHash = BytesToScVal(await uploadContract('InterchainToken', options, wallet, chain));
            const tokenManagerWasmHash = BytesToScVal(await uploadContract('TokenManager', options, wallet, chain));

            return {
                owner,
                operator,
                gatewayAddress,
                gasServiceAddress,
                itsHubAddress,
                chainName,
                nativeTokenAddress,
                interchainTokenWasmHash,
                tokenManagerWasmHash,
            };
        }

        case 'AxelarOperators':
            return { owner };

        case 'AxelarGasService': {
            const operatorsAddress = chain.contracts?.AxelarOperators?.address;

            validateParameters({
                isValidStellarAddress: { operatorsAddress },
            });

            const operator = operatorsAddress ? nativeToScVal(Address.fromString(operatorsAddress), { type: 'address' }) : owner;

            return { owner, operator };
        }

        case 'Upgrader': {
            return {};
        }

        case 'AxelarExample': {
            const gatewayAddress = nativeToScVal(Address.fromString(chain.contracts?.AxelarGateway?.address), { type: 'address' });
            const gasServiceAddress = nativeToScVal(Address.fromString(chain.contracts?.AxelarGasService?.address), { type: 'address' });
            const itsAddress = options.useDummyItsAddress
                ? gatewayAddress
                : nativeToScVal(chain.contracts?.InterchainTokenService?.address, { type: 'address' });

            return { gatewayAddress, gasServiceAddress, itsAddress };
        }

        case 'Multicall': {
            return {};
        }

        default:
            throw new Error(`Unknown contract: ${contractName}`);
    }
}

async function deploy(options, config, chain, contractName) {
    const { yes } = options;
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    const wasmHash = await uploadWasm(wallet, chain, options.contractCodePath);
    const initializeArgs = await getInitializeArgs(config, chain, contractName, wallet, options);
    const serializedArgs = Object.fromEntries(
        Object.entries(initializeArgs).map(([key, value]) => [key, serializeValue(scValToNative(value))]),
    );
    const operation = Operation.createCustomContract({
        wasmHash,
        address: Address.fromString(wallet.publicKey()),
        // requires that initializeArgs returns the parameters in the appropriate order
        constructorArgs: Object.values(initializeArgs),
    });
    printInfo('Initializing contract with args', JSON.stringify(serializedArgs, null, 2));

    const deployResponse = await broadcast(operation, wallet, chain, 'Initialized contract', options);
    const contractAddress = Address.fromScAddress(deployResponse.address()).toString();

    validateParameters({
        isValidStellarAddress: { contractAddress },
    });

    printInfo('Contract initialized at address', contractAddress);

    chain.contracts[contractName] = {
        address: contractAddress,
        deployer: wallet.publicKey(),
        wasmHash: serializeValue(wasmHash),
        initializeArgs: serializedArgs,
    };

    printInfo(contractName, JSON.stringify(chain.contracts[contractName], null, 2));
}

async function uploadWasm(wallet, chain, filePath) {
    const bytecode = readFileSync(filePath);
    const operation = Operation.uploadContractWasm({ wasm: bytecode });
    const wasmResponse = await broadcast(operation, wallet, chain, 'Uploaded wasm');
    return wasmResponse.value();
}

async function upgrade(options, _, chain, contractName) {
    const { yes } = options;

    if (!options.version && !options.artifactPath) {
        throw new Error('--version or --artifact-path required to upgrade');
    }

    let contractAddress = chain.contracts[contractName]?.address;
    const upgraderAddress = chain.contracts.Upgrader?.address;
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with upgrade on ${chain.name}?`, yes)) {
        return;
    }

    validateParameters({
        isValidStellarAddress: { contractAddress, upgraderAddress },
    });

    contractAddress = Address.fromString(contractAddress);

    const newWasmHash = await uploadWasm(wallet, chain, options.contractCodePath);
    printInfo('New Wasm hash', serializeValue(newWasmHash));

    const args = [contractAddress, options.version, newWasmHash, [options.migrationData]].map(nativeToScVal);

    const upgrader = new Contract(upgraderAddress);
    const operation = upgrader.call('upgrade', ...args);

    await broadcast(operation, wallet, chain, 'Upgraded contract', options);
    chain.contracts[contractName].wasmHash = serializeValue(newWasmHash);
    printInfo('Contract upgraded successfully!', contractAddress);
}

async function mainProcessor(options, processor, contractName) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);

    if (!chain.contracts) {
        chain.contracts = {};
    }

    await processor(options, config, chain, contractName);
    saveConfig(config, options.env);
}

function main() {
    // 1st level command
    const program = new Command('deploy-contract').description('Deploy/Upgrade Stellar contracts');

    // 2nd level commands
    const deployCmd = new Command('deploy').description('Deploy a Stellar contract');
    const upgradeCmd = new Command('upgrade').description('Upgrade a Stellar contract');

    // 3rd level commands for `deploy`
    const deployContractCmds = Array.from(SUPPORTED_CONTRACTS).map((contractName) => {
        const command = new Command(contractName).description(`Deploy ${contractName} contract`);

        addStoreOptions(command);
        addDeployOptions(command);

        // Attach the preAction hook to this specific command
        command.hook('preAction', preActionHook(contractName));

        // Main action handler
        command.action((options) => {
            mainProcessor(options, deploy, contractName);
        });

        return command;
    });

    // 3rd level commands for `upgrade`
    const upgradeContractCmds = Array.from(SUPPORTED_CONTRACTS).map((contractName) => {
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

        command.hook('preAction', async (thisCommand) => {
            const opts = thisCommand.opts();

            const contractCodePath = await getContractCodePath(opts, contractName);
            Object.assign(opts, { contractCodePath });
        });

        command.action((options) => {
            options.migrationData = sanitizeMigrationData(options.migrationData, options.version, contractName);
            mainProcessor(options, upgrade, contractName);
        });

        return command;
    });

    // Add 3rd level commands to 2nd level command `deploy`
    deployContractCmds.forEach((cmd) => deployCmd.addCommand(cmd));

    // Add 3rd level commands to 2nd level command `upgrade`
    upgradeContractCmds.forEach((cmd) => upgradeCmd.addCommand(cmd));

    // Add base options to all 3rd level commands
    addOptionsToCommands(deployCmd, addBaseOptions);
    addOptionsToCommands(upgradeCmd, addBaseOptions);

    // Add 2nd level commands to 1st level command
    program.addCommand(deployCmd);
    program.addCommand(upgradeCmd);

    program.parse();
}

function preActionHook(contractName) {
    return async (thisCommand) => {
        const opts = thisCommand.opts();

        // Pass contractName directly since it's known in this scope
        const contractCodePath = await getContractCodePath(opts, contractName);
        Object.assign(opts, { contractCodePath });
    };
}

function sanitizeMigrationData(migrationData, version, contractName) {
    if (migrationData === null || migrationData === '()') return null;

    try {
        return Address.fromString(migrationData);
    } catch (_) {
        // not an address, continue to next parsing attempt
    }

    let parsed;

    try {
        parsed = JSON.parse(migrationData);
    } catch (_) {
        // not json, keep as string
        return migrationData;
    }

    if (Array.isArray(parsed)) {
        return parsed.map((value) => sanitizeMigrationData(value, version, contractName));
    }

    const custom = customMigrationData(parsed, version, contractName);

    if (custom) {
        return custom;
    }

    if (parsed !== null && typeof parsed === 'object') {
        return Object.fromEntries(Object.entries(parsed).map(([key, value]) => [key, sanitizeMigrationData(value, version, contractName)]));
    }

    printInfo('Sanitized migration data', parsed);

    return parsed;
}

function customMigrationData(migrationDataObj, version, contractName) {
    if (!version || !VERSIONED_CUSTOM_MIGRATION_DATA_TYPES[version] || !VERSIONED_CUSTOM_MIGRATION_DATA_TYPES[version][contractName]) {
        return null;
    }

    const customMigrationDataTypeToScVal = VERSIONED_CUSTOM_MIGRATION_DATA_TYPES[version][contractName];

    try {
        printInfo(`Retrieving custom migration data for ${contractName}`);
        return customMigrationDataTypeToScVal(migrationDataObj);
    } catch (error) {
        throw new Error(`Failed to convert custom migration data for ${contractName}: ${error}`);
    }
}

if (require.main === module) {
    main();
}
