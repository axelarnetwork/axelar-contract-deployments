'use strict';

const { Address, nativeToScVal, scValToNative, Operation, Contract } = require('@stellar/stellar-sdk');
const { loadConfig, printInfo, saveConfig } = require('../../evm/utils');
const { getWallet, broadcast, serializeValue, getContractCodePath, BytesToScVal, getUploadContractCodePath } = require('../utils');
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
        ...(options.version && { version: options.version }),
        initializeArgs: serializedArgs,
    };

    printInfo('Contract deployed successfully', chain.contracts[contractName]);
};

const upgrade = async (options, _, chain, contractName) => {
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

    const newWasmHash = await uploadWasm(wallet, chain, options.contractCodePath, contractName);
    printInfo('New Wasm hash', serializeValue(newWasmHash));

    const args = [contractAddress, options.version, newWasmHash, [options.migrationData]].map(nativeToScVal);

    const upgrader = new Contract(upgraderAddress);
    const operation = upgrader.call('upgrade', ...args);

    await broadcast(operation, wallet, chain, 'Upgraded contract', options);
    chain.contracts[contractName].wasmHash = serializeValue(newWasmHash);

    if (options.version) {
        chain.contracts[contractName].version = options.version;
    }

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

        case 'TokenUtils': {
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

const mainProcessor = async (options, processor, contractName) => {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);

    if (!chain.contracts) {
        chain.contracts = {};
    }

    await processor(options, config, chain, contractName);
    saveConfig(config, options.env);
};

module.exports = {
    deploy,
    upgrade,
    upload,
    mainProcessor,
};
