'use strict';

const { Address, nativeToScVal, scValToNative, Operation, StrKey } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { getWallet, broadcast, serializeValue, addBaseOptions } = require('./utils');
const { getDomainSeparator, getChainConfig } = require('../common');
const { prompt, validateParameters } = require('../common/utils');
const { weightedSignersToScVal } = require('./type-utils');
const { ethers } = require('hardhat');
const { readFileSync } = require('fs');
const {
    utils: { arrayify, id },
} = ethers;
require('./cli-utils');

async function getInitializeArgs(config, chain, contractName, wallet, options) {
    const owner = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });
    const operator = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });

    switch (contractName) {
        case 'axelar_gateway': {
            const domainSeparator = nativeToScVal(Buffer.from(arrayify(await getDomainSeparator(config, chain, options))));
            const minimumRotationDelay = nativeToScVal(0);
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

        case 'interchain_token_service':
            return { owner };

        case 'axelar_operators':
            return { owner };

        case 'axelar_gas_service': {
            const operatorsAddress = chain?.contracts?.axelar_operators?.address;
            const gasCollector = operatorsAddress ? nativeToScVal(Address.fromString(operatorsAddress), { type: 'address' }) : owner;

            return { gasCollector };
        }

        case 'example': {
            const gatewayAddress = nativeToScVal(Address.fromString(chain?.contracts?.axelar_gateway?.address), { type: 'address' });
            const gasServiceAddress = nativeToScVal(Address.fromString(chain?.contracts?.axelar_gas_service?.address), { type: 'address' });

            return { gatewayAddress, gasServiceAddress };
        }

        default:
            throw new Error(`Unknown contract: ${contractName}`);
    }
}

async function deploy(options, config, chain, contractName) {
    const { wasmPath, yes } = options;
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    const initializeArgs = await getInitializeArgs(config, chain, contractName, wallet, options);
    const serializedArgs = Object.fromEntries(
        Object.entries(initializeArgs).map(([key, value]) => [key, serializeValue(scValToNative(value))]),
    );
    const wasmHash = await uploadWasm(wasmPath, wallet, chain);

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
    };
}

async function uploadWasm(filePath, wallet, chain) {
    const bytecode = readFileSync(filePath);
    const operation = Operation.uploadContractWasm({ wasm: bytecode });
    const wasmResponse = await broadcast(operation, wallet, chain, 'Uploaded wasm');
    return wasmResponse.value();
}

async function upgrade(options, _, chain, contractName) {
    const { wasmPath, yes } = options;
    const contractAddress = chain.contracts[contractName].address;
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with upgrade on ${chain.name}?`, yes)) {
        return;
    }

    validateParameters({
        isNonEmptyString: { contractAddress },
    });

    const newWasmHash = await uploadWasm(wasmPath, wallet, chain);
    printInfo('New Wasm hash', serializeValue(newWasmHash));

    const operation = Operation.invokeContractFunction({
        contract: contractAddress,
        function: 'upgrade',
        args: [nativeToScVal(newWasmHash)],
    });
    await broadcast(operation, wallet, chain, 'Upgraded contract', options);
    chain.contracts[contractName].wasmHash = newWasmHash;
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

    // 2nd level deploy command
    const deployCmd = new Command('deploy')
        .description('Deploy a Stellar contract')
        .argument('<contract-name>', 'contract name to deploy')
        .addOption(new Option('--wasm-path <wasmPath>', 'path to the WASM file').makeOptionMandatory(true))
        .addOption(new Option('--nonce <nonce>', 'optional nonce for the signer set'))
        .addOption(new Option('--domain-separator <domainSeparator>', 'domain separator (keccak256 hash or "offline")').default('offline'))
        .addOption(
            new Option('--previous-signers-retention <previousSignersRetention>', 'previous signer retention')
                .default(15)
                .argParser(Number),
        )
        .action((contractName, options) => {
            mainProcessor(options, deploy, contractName);
        });

    // 2nd level upgrade command
    const upgradeCmd = new Command('upgrade')
        .description('Upgrade a Stellar contract')
        .argument('<contract-name>', 'contract name to deploy')
        .addOption(new Option('--wasm-path <wasmPath>', 'path to the WASM file'))
        .action((contractName, options) => {
            mainProcessor(options, upgrade, contractName);
        });

    // Add base options to all 2nd level commands
    addBaseOptions(upgradeCmd, { address: true });
    addBaseOptions(deployCmd, { address: true });

    // Add 2nd level commands to 1st level command
    program.addCommand(deployCmd);
    program.addCommand(upgradeCmd);

    program.parse();
}

if (require.main === module) {
    main();
}
