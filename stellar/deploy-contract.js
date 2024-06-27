'use strict';

const { Contract, Address, nativeToScVal, scValToNative } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { execSync } = require('child_process');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { getNetworkPassphrase, getWallet, prepareTransaction, sendTransaction, buildTransaction, estimateCost } = require('./utils');
require('./cli-utils');

function getInitializeArgs(chain, contractName, wallet, options) {
    const address = Address.fromString(wallet.publicKey());

    switch (contractName) {
        case 'axelar_gateway': {
            const authAddress = chain.contracts?.axelar_auth_verifiers?.address;

            if (!authAddress) {
                throw new Error('Missing axelar_auth_verifiers contract address');
            }

            return [nativeToScVal(authAddress, { type: 'address' }), nativeToScVal(address, { type: 'address' })];
        }

        case 'axelar_auth_verifiers': {
            const owner = nativeToScVal(address, { type: 'address' });
            const previousSignersRetention = nativeToScVal(15, { type: 'u64' });
            const domainSeparator = nativeToScVal(Buffer.alloc(32));
            const minimumRotationDelay = nativeToScVal(0, { type: 'u64' });

            // Create a vector of WeightedSigners
            const initialSigners = nativeToScVal(
                [
                    {
                        nonce: Buffer.alloc(32),
                        signers: [
                            {
                                signer: Address.fromString(wallet.publicKey()).toBuffer(),
                                weight: 1,
                            },
                        ],
                        threshold: 1,
                    },
                ],
                {
                    type: {
                        signers: [
                            'symbol',
                            {
                                signer: ['symbol', 'bytes'],
                                weight: ['symbol', 'u256'],
                            },
                        ],
                        nonce: ['symbol', 'bytes'],
                        threshold: ['symbol', 'u256'],
                    },
                },
            );

            return [owner, previousSignersRetention, domainSeparator, minimumRotationDelay, initialSigners];
        }

        case 'axelar_operators':
            return [address];
        default:
            throw new Error(`Unknown contract: ${contractName}`);
    }
}

async function processCommand(options, config, chain) {
    const { wasmPath, contractName } = options;

    const { rpc, networkType } = chain;
    const networkPassphrase = getNetworkPassphrase(networkType);
    const [wallet, server] = await getWallet(chain, options);

    const cmd = `soroban contract deploy --wasm ${wasmPath} --source ${options.privateKey} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}"`;
    printInfo('Deploying contract', contractName);

    let contractAddress = options.address; // || chain.contracts?.axelar_auth_verifiers?.address;

    if (!contractAddress) {
        contractAddress = execSync(cmd, { encoding: 'utf-8', stdio: 'pipe' }).trimEnd();
        printInfo('Deployed contract successfully!', contractAddress);
    } else {
        printInfo('Using existing contract', contractAddress);
    }

    chain.contracts[contractName] = {
        address: contractAddress,
        deployer: wallet.publicKey(),
    };

    if (!options.initialize) {
        return;
    }

    function serializeValue(value) {
        if (value instanceof Uint8Array) {
            return Buffer.from(value).toString('hex');
        }

        if (Array.isArray(value)) {
            return value.map(serializeValue);
        }

        if (typeof value === 'bigint') {
            return value.toString();
        }

        if (typeof value === 'object') {
            return Object.entries(value).reduce((acc, [key, val]) => {
                acc[key] = serializeValue(val);
                return acc;
            }, {});
        }

        return value;
    }

    function printValue(value) {
        if (Array.isArray(value)) {
            return JSON.stringify(value.map(printValue));
        }

        return value.toString();
    }

    const initializeArgs = getInitializeArgs(chain, contractName, wallet, options);
    chain.contracts[contractName].initializeArgs = initializeArgs.map(scValToNative).map(serializeValue);

    const contract = new Contract(contractAddress);
    const operation = contract.call('initialize', ...initializeArgs); // ...initializeArgs.map((arg) => arg.toScVal()));

    printInfo('Initializing contract with args', initializeArgs.map(scValToNative).map(serializeValue).map(printValue));

    if (options.estimateCost) {
        const tx = await buildTransaction(operation, server, wallet, chain.networkType, options);
        const resourceCost = await estimateCost(tx, server);
        printInfo('Resource cost', JSON.stringify(resourceCost, null, 2));
        return;
    }

    const preparedTx = await prepareTransaction(operation, server, wallet, networkType, options);
    const returnValue = await sendTransaction(preparedTx, server);

    printInfo('Contract initialized', returnValue);
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    await processor(options, config, config.stellar);
    saveConfig(config, options.env);
}

function main() {
    const program = new Command();
    program.name('deploy-contract').description('Deploy Axelar Soroban contracts on Stellar');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').env('PRIVATE_KEY'));
    program.addOption(new Option('-v, --verbose', 'verbose output').default(false));
    program.addOption(new Option('--initialize', 'initialize the contract'));
    program.addOption(new Option('--contractName <contractName>', 'contract name to deploy').makeOptionMandatory(true));
    program.addOption(new Option('--wasmPath <wasmPath>', 'path to the WASM file').makeOptionMandatory(true));
    program.addOption(new Option('--address <address>', 'existing instance to initialize'));
    program.addOption(new Option('--estimateCost', 'estimate on-chain resources').default(false));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}

if (require.main === module) {
    main();
}
