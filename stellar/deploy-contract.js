'use strict';

const { Address, nativeToScVal, scValToNative, Operation, StrKey, xdr, authorizeInvocation, rpc } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { getWallet, broadcast, serializeValue, addBaseOptions, getNetworkPassphrase, createAuthorizedFunc, getWasmFilePath, SUPPORTED_STELLAR_CONTRACTS } = require('./utils');
const { getDomainSeparator, getChainConfig, addOptionsToCommands } = require('../common');
const { prompt, validateParameters } = require('../common/utils');
const { weightedSignersToScVal } = require('./type-utils');
const { ethers } = require('hardhat');
const { readFileSync } = require('fs');
const {
    utils: { arrayify, id },
} = ethers;
require('./cli-utils');

const CONTRACT_CONFIGS = {
    axelar_gateway: () => [
        new Option('--nonce <nonce>', 'optional nonce for the signer set'),
        new Option('--domain-separator <domainSeparator>', 'domain separator (keccak256 hash or "offline")').default('offline'),
        new Option('--previous-signers-retention <previousSignersRetention>', 'previous signer retention').default(15).argParser(Number),
        new Option('--minimum-rotation-delay <miniumRotationDelay>', 'minimum rotation delay').default(0).argParser(Number),
    ],
    example: () => [new Option('--use-dummy-its-address', 'use dummy its address for example contract to test a GMP call').default(false)],
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

async function getInitializeArgs(config, chain, contractName, wallet, options) {
    const owner = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });
    const operator = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });

    switch (contractName) {
        case 'axelar_gateway': {
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

        case 'interchain_token_service': {
            const gatewayAddress = nativeToScVal(Address.fromString(chain.contracts?.axelar_gateway?.address), { type: 'address' });
            const gasServiceAddress = nativeToScVal(Address.fromString(chain.contracts?.axelar_gas_service?.address), { type: 'address' });
            const itsHubAddress = nativeToScVal(config.axelar?.contracts?.InterchainTokenService?.address, { type: 'string' });
            const chainName = nativeToScVal(chain.axelarId, { type: 'string' });
            const nativeTokenAddress = nativeToScVal(Address.fromString(chain?.tokenAddress), { type: 'address' });

            if (!chain.contracts?.interchain_token?.wasmHash) {
                throw new Error(`interchain_token contract's wasm hash does not exist.`);
            }

            const interchainTokenWasmHash = nativeToScVal(Buffer.from(chain.contracts?.interchain_token?.wasmHash, 'hex'), {
                type: 'bytes',
            });

            const tokenManagerWasmHash = nativeToScVal(Buffer.from(chain.contracts?.token_manager?.wasmHash, 'hex'), {
                type: 'bytes',
            });

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

        case 'axelar_operators':
            return { owner };

        case 'axelar_gas_service': {
            const operatorsAddress = chain.contracts?.axelar_operators?.address;
            const operator = operatorsAddress ? nativeToScVal(Address.fromString(operatorsAddress), { type: 'address' }) : owner;

            return { owner, operator };
        }

        case 'upgrader': {
            return {};
        }

        case 'example': {
            const gatewayAddress = nativeToScVal(Address.fromString(chain.contracts?.axelar_gateway?.address), { type: 'address' });
            const gasServiceAddress = nativeToScVal(Address.fromString(chain.contracts?.axelar_gas_service?.address), { type: 'address' });
            const itsAddress = options.useDummyItsAddress
                ? gatewayAddress
                : nativeToScVal(chain.contracts?.interchain_token_service?.address, { type: 'address' });

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

    const wasmHash = await uploadWasm(options.wasmResolvedPath, wallet, chain);

    if (contractName === 'interchain_token' || contractName === 'token_manager') {
        chain.contracts[contractName] = {
            deployer: wallet.publicKey(),
            wasmHash: serializeValue(wasmHash),
        };
    } else {
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
        const contractAddress = StrKey.encodeContract(Address.fromScAddress(deployResponse.address()).toBuffer());

        printInfo('Contract initialized at address', contractAddress);

        chain.contracts[contractName] = {
            address: contractAddress,
            deployer: wallet.publicKey(),
            wasmHash: serializeValue(wasmHash),
            initializeArgs: serializedArgs,
        };
    }

    printInfo(contractName, JSON.stringify(chain.contracts[contractName], null, 2));
}

async function uploadWasm(filePath, wallet, chain) {
    const bytecode = readFileSync(filePath);
    const operation = Operation.uploadContractWasm({ wasm: bytecode });
    const wasmResponse = await broadcast(operation, wallet, chain, 'Uploaded wasm');
    return wasmResponse.value();
}

async function upgrade(options, _, chain, contractName) {
    const { wasmPath, yes } = options;
    let contractAddress = chain.contracts[contractName]?.address;
    const upgraderAddress = chain.contracts.upgrader?.address;
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with upgrade on ${chain.name}?`, yes)) {
        return;
    }

    validateParameters({
        isNonEmptyString: { contractAddress, upgraderAddress },
    });

    contractAddress = Address.fromString(contractAddress);

    const newWasmHash = await uploadWasm(wasmPath, wallet, chain);
    printInfo('New Wasm hash', serializeValue(newWasmHash));

    const operation = Operation.invokeContractFunction({
        contract: chain.contracts.upgrader.address,
        function: 'upgrade',
        args: [contractAddress, options.newVersion, newWasmHash, [options.migrationData]].map(nativeToScVal),
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

    // 2nd level deploy command
    const deployCmd = new Command('deploy').description('Deploy a Stellar contract');

    // 2nd level upgrade command
    const upgradeCmd = new Command('upgrade').description('Upgrade a Stellar contract');

    // Add base options to all 2nd level commands
    addBaseOptions(upgradeCmd, { address: true });
    addBaseOptions(deployCmd, { address: true });

    // 3rd level commands for `deploy`
    const deployContractCmds = Array.from(SUPPORTED_STELLAR_CONTRACTS).map((contractName) => {
        const command = new Command(contractName)
            .description(`Deploy ${contractName} contract`)
            .addOption(
                new Option('--wasm-path <wasmPath>', 'Path to the WASM file (required if --version is not used)')
                    .conflicts('version'),
            )
            .addOption(
                new Option('--version <version>', 'Released version (vX.Y.Z) or a pre-release commit hash (required if --wasm-path is not used)')
                    .conflicts('wasmPath'),
            )
            .hook('preAction', async (thisCommand) => {
                const opts = thisCommand.opts();
    
                if (!opts.wasmPath && !opts.version) {
                    throw new Error('Either --wasm-path or --version is required');
                }
    
                const wasmResolvedPath = await getWasmFilePath(opts, contractName);
                Object.assign(opts, { wasmResolvedPath });
            })
            .action((contractName, options) => {
                mainProcessor(options, deploy, contractName);
            });

        return addDeployOptions(command);
    });

    // 3rd level commands for `upgrade`
    const upgradeContractCmds = Array.from(SUPPORTED_STELLAR_CONTRACTS).map((contractName) => {
        return new Command(contractName)
            .description(`Upgrade ${contractName} contract`)
            .addOption(new Option('--wasm-path <wasmPath>', 'path to the WASM file'))
            .addOption(new Option('--new-version <newVersion>', 'new version of the contract'))
            .addOption(new Option('--migration-data <migrationData>', 'migration data').default('()'))
            .action((options) => {
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

if (require.main === module) {
    main();
}
