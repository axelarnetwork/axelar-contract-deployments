'use strict';

const { Address, nativeToScVal, scValToNative, Operation, xdr, authorizeInvocation, rpc } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const {
    getWallet,
    broadcast,
    serializeValue,
    addBaseOptions,
    getNetworkPassphrase,
    createAuthorizedFunc,
    BytesToScVal,
    pascalToKebab,
} = require('./utils');
const { getDomainSeparator, getChainConfig, addOptionsToCommands } = require('../common');
const { prompt, validateParameters } = require('../common/utils');
const { weightedSignersToScVal } = require('./type-utils');
const { ethers } = require('hardhat');
const { writeFileSync, readFileSync } = require('fs');
const fetch = require('node-fetch');
const path = require('path');
const {
    utils: { arrayify, id },
} = ethers;
require('./cli-utils');

const AXELAR_RELEASE_BASE_URL = 'https://static.axelar.network/releases/stellar';

const SUPPORTED_CONTRACTS = new Set([
    'AxelarExample',
    'AxelarGateway',
    'AxelarOperators',
    'AxelarGasService',
    'InterchainToken',
    'TokenManager',
    'InterchainTokenService',
    'Upgrader',
]);

const CONTRACT_CONFIGS = {
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

const addDeployOptions = (program) => {
    // Get the package name from the program name
    const contractName = program.name();
    // Find the corresponding options for the package
    const cmdOptions = CONTRACT_CONFIGS[contractName];

    if (cmdOptions) {
        const options = cmdOptions();
        // Add the options to the program
        options.forEach((option) => program.addOption(option));
    }

    return program;
};

function getWasmUrl(contractName, version) {
    if (!SUPPORTED_CONTRACTS.has(contractName)) {
        throw new Error(`Unsupported contract ${contractName} for versioned deployment`);
    }

    // Convert `AxelarGateway` → `stellar-axelar-gateway`
    const dirPath = `stellar-${pascalToKebab(contractName)}`;
    const fileName = dirPath.replace(/-/g, '_');

    return `${AXELAR_RELEASE_BASE_URL}/${dirPath}/${version}/wasm/${fileName}.wasm`;
}

async function downloadWasmFile(contractName, version) {
    const url = getWasmUrl(contractName, version);
    const tempDir = path.join(process.cwd(), 'artifacts');

    // Create temp directory if it doesn't exist
    const fs = require('fs');

    if (!fs.existsSync(tempDir)) {
        fs.mkdirSync(tempDir, { recursive: true });
    }

    const outputPath = path.join(tempDir, `${contractName}-${version}.wasm`);

    try {
        const response = await fetch(url);

        if (!response.ok) {
            throw new Error(`Failed to download WASM file: ${response.statusText}`);
        }

        const buffer = await response.buffer();
        writeFileSync(outputPath, buffer);
        printInfo('Successfully downloaded WASM file', { contractName, outputPath });
        return outputPath;
    } catch (error) {
        throw new Error(`Error downloading WASM file: ${error.message}`);
    }
}

async function getWasmFile(options, contractName) {
    if (options.wasmPath) {
        return options.wasmPath;
    }

    if (options.version) {
        printInfo(`Downloading WASM file`, { version: options.version, contractName });
        return await downloadWasmFile(contractName, options.version);
    }

    throw new Error('Either --wasm-path or --version must be provided');
}

async function uploadContract(contractName, options, wallet, chain) {
    const wasmFile = await getWasmFile(options, contractName);
    return await uploadWasm(wasmFile, wallet, chain);
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

    const wasmHash = await uploadContract(contractName, options, wallet, chain);
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

async function uploadWasm(filePath, wallet, chain) {
    const bytecode = readFileSync(filePath);
    const operation = Operation.uploadContractWasm({ wasm: bytecode });
    const wasmResponse = await broadcast(operation, wallet, chain, 'Uploaded wasm');
    return wasmResponse.value();
}

async function upgrade(options, _, chain, contractName) {
    const { yes } = options;

    if (!options.version && !options.wasmPath) {
        throw new Error('--version or --wasm-path required to upgrade');
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

    const wasmFile = await getWasmFile(options, contractName);
    const newWasmHash = await uploadWasm(wasmFile, wallet, chain);
    printInfo('New Wasm hash', serializeValue(newWasmHash));

    const version = sanitizeUpgradeVersion(options.version);

    const operation = Operation.invokeContractFunction({
        contract: chain.contracts.Upgrader.address,
        function: 'upgrade',
        args: [contractAddress, version, newWasmHash, [options.migrationData]].map(nativeToScVal),
        auth: await createUpgradeAuths(contractAddress, newWasmHash, options.migrationData, chain, wallet),
    });

    await broadcast(operation, wallet, chain, 'Upgraded contract', options);
    chain.contracts[contractName].wasmHash = serializeValue(newWasmHash);
    printInfo('Contract upgraded successfully!', contractAddress);
}

async function createUpgradeAuths(contractAddress, newWasmHash, migrationData, chain, wallet) {
    // 20 seems a reasonable number of ledgers to allow for the upgrade to take effect
    const validUntil = await new rpc.Server(chain.rpc).getLatestLedger().then((info) => info.sequence + 20);

    return Promise.all(
        [
            createAuthorizedFunc(contractAddress, 'upgrade', [nativeToScVal(newWasmHash)]),
            createAuthorizedFunc(contractAddress, 'migrate', [nativeToScVal(migrationData)]),
        ].map((auth) =>
            authorizeInvocation(
                wallet,
                validUntil,
                new xdr.SorobanAuthorizedInvocation({
                    function: auth,
                    subInvocations: [],
                }),
                wallet.publicKey(),
                getNetworkPassphrase(chain.networkType),
            ),
        ),
    );
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
        const command = new Command(contractName)
            .description(`Deploy ${contractName} contract`)
            .addOption(new Option('--wasm-path <wasmPath>', 'path to the WASM file'))
            .addOption(new Option('--version <version>', 'version of the contract to deploy (e.g., v1.0.0)'))
            .action((options) => {
                mainProcessor(options, deploy, contractName);
            });

        return addDeployOptions(command);
    });

    // 3rd level commands for `upgrade`
    const upgradeContractCmds = Array.from(SUPPORTED_CONTRACTS).map((contractName) => {
        return new Command(contractName)
            .description(`Upgrade ${contractName} contract`)
            .addOption(new Option('--wasm-path <wasmPath>', 'path to the WASM file'))
            .addOption(new Option('--version <version>', 'new version of the contract to upgrade to (e.g., v1.1.0)'))
            .addOption(new Option('--migration-data <migrationData>', 'migration data').default(null, '()'))
            .addHelpText(
                'after',
                `
Examples:
  # using Vec<Address> as migration data:
  $ deploy-contract upgrade axelar-operators deploy --wasm-path {releasePath}/stellar_axelar_operators.optimized.wasm --version 2.1.7 --migration-data '["GDYBNA2LAWDKRSCIR4TKCB5LJCDRVUWKHLMSKUWMJ3YX3BD6DWTNT5FW"]'

  # default void migration data:
  $ deploy-contract upgrade axelar-gateway deploy --wasm-path {releasePath}/stellar_axelar_gateway.optimized.wasm --version 1.0.1

  # equivalent explicit void migration data:
  $ deploy-contract upgrade axelar-gateway deploy --wasm-path {releasePath}/stellar_axelar_gateway.optimized.wasm --version 1.0.1 --migration-data '()'
`,
            )
            .action((options) => {
                options.migrationData = sanitizeMigrationData(options.migrationData);
                mainProcessor(options, upgrade, contractName);
            });
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

function sanitizeMigrationData(migrationData) {
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
        return parsed.map(sanitizeMigrationData);
    }

    if (parsed !== null && typeof parsed === 'object') {
        return Object.fromEntries(Object.entries(parsed).map(([key, value]) => [key, sanitizeMigrationData(value)]));
    }

    return parsed;
}

/* Note: Once R2 uploads for stellar use the cargo version number (does not include 'v' prefix), this will no longer be necessary. */
function sanitizeUpgradeVersion(version) {
    if (version.startsWith('v')) {
        return version.slice(1);
    }

    return version;
}

if (require.main === module) {
    main();
}
