'use strict';

const { Address, nativeToScVal, scValToNative, Operation, authorizeInvocation, xdr, rpc } = require('@stellar/stellar-sdk');
const { loadConfig, printInfo, saveConfig } = require('../../evm/utils');
const {
    getWallet,
    broadcast,
    serializeValue,
    getContractCodePath,
    BytesToScVal,
    getUploadContractCodePath,
    createAuthorizedFunc,
    getNetworkPassphrase,
    getContractVersion,
} = require('../utils');
const { getDomainSeparator, getChainConfig } = require('../../common');
const { prompt, validateParameters } = require('../../common/utils');
const { weightedSignersToScVal } = require('../type-utils');
const { ethers } = require('hardhat');
const { readFileSync } = require('fs');
const {
    utils: { arrayify, id },
} = ethers;

require('../cli-utils');

const deploy = async (options, config, chain, contractName) => {
    const { yes } = options;
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    const wasmHash = await uploadWasm(wallet, chain, options.contractCodePath, contractName);
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

    updateContractVersionInfo(chain, contractName, options);

    printInfo('Contract deployed successfully', chain.contracts[contractName]);
};

const upgrade = async (options, _, chain, contractName) => {
    const { yes } = options;

    if (!options.version && !options.artifactPath) {
        throw new Error('--version or --artifact-path required to upgrade');
    }

    if (contractName === 'InterchainTokenService') {
        if (!options.interchainTokenVersion || !options.tokenManagerVersion) {
            throw new Error('--interchain-token-version and --token-manager-version are required to upgrade InterchainTokenService');
        }
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

    const newWasmHash = await uploadWasm(wallet, chain, options.contractCodePath, contractName);
    printInfo('New Wasm hash', serializeValue(newWasmHash));

    // TODO: Revert this after v1.1.1 release
    const version = sanitizeUpgradeVersion(options.version);

    // TODO: Revert this after v1.1.1 release
    const operation = Operation.invokeContractFunction({
        contract: chain.contracts.Upgrader.address,
        function: 'upgrade',
        args: [contractAddress, version, newWasmHash, [options.migrationData]].map(nativeToScVal),
        auth: await createUpgradeAuths(contractAddress, newWasmHash, options.migrationData, chain, wallet),
    });

    await broadcast(operation, wallet, chain, 'Upgraded contract', options);
    chain.contracts[contractName].wasmHash = serializeValue(newWasmHash);
    updateContractVersionInfo(chain, contractName, options);
    updateInterchainTokenServiceWasmHash(chain, contractName, options);

    printInfo('Contract upgraded successfully', { contractName, newWasmHash: serializeValue(newWasmHash) });
};

const upload = async (options, _, chain, contractName) => {
    const wallet = await getWallet(chain, options);
    const contractCodePath = await getUploadContractCodePath(options, contractName);
    const newWasmHash = await uploadWasm(wallet, chain, contractCodePath, contractName);
    printInfo('Contract uploaded successfully', { contractName, wasmHash: serializeValue(newWasmHash) });
};

const getInitializeArgs = async (config, chain, contractName, wallet, options) => {
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
};

const uploadContract = async (contractName, options, wallet, chain) => {
    const contractCodePath = await getContractCodePath(options, contractName);
    return uploadWasm(wallet, chain, contractCodePath, contractName);
};

const uploadWasm = async (wallet, chain, filePath, contractName) => {
    const bytecode = readFileSync(filePath);
    const operation = Operation.uploadContractWasm({ wasm: bytecode });
    const wasmResponse = await broadcast(operation, wallet, chain, `Uploaded ${contractName} wasm`);
    return wasmResponse.value();
};

const updateContractVersionInfo = (chain, contractName, options) => {
    chain.contracts[contractName].version = options.version;

    if (contractName !== 'InterchainTokenService') return;

    chain.contracts[contractName].interchainTokenVersion = getContractVersion(options, 'InterchainToken');
    chain.contracts[contractName].tokenManagerVersion = getContractVersion(options, 'TokenManager');
};

const updateInterchainTokenServiceWasmHash = (chain, contractName, options) => {
    if (contractName !== 'InterchainTokenService') return;

    const migrationData = scValToNative(options.migrationData);

    chain.contracts[contractName].initializeArgs.interchainTokenWasmHash = serializeValue(migrationData.new_interchain_token_wasm_hash);
    chain.contracts[contractName].initializeArgs.tokenManagerWasmHash = serializeValue(migrationData.new_token_manager_wasm_hash);
};

const mainProcessor = async (options, processor, contractName) => {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);

    if (!chain.contracts) {
        chain.contracts = {};
    }

    await processor(options, config, chain, contractName);
    saveConfig(config, options.env);
};

// TODO: Remove this after v1.1.1 release
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

// TODO: Remove this after v1.1.1 release
/* Note: Once R2 uploads for stellar use the cargo version number (does not include 'v' prefix), this will no longer be necessary. */
function sanitizeUpgradeVersion(version) {
    if (version.startsWith('v')) {
        return version.slice(1);
    }

    return version;
}

module.exports = {
    deploy,
    upgrade,
    upload,
    mainProcessor,
};
