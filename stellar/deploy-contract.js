'use strict';

const { Contract, Address, ScInt, nativeToScVal } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { execSync } = require('child_process');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { getNetworkPassphrase, getWallet, prepareTransaction, sendTransaction, buildTransaction, estimateCost } = require('./utils');
const { addEnvOption } = require('../common');
require('./cli-utils');

function getInitializeArgs(chain, contractName, wallet, options) {
    const address = Address.fromString(wallet.publicKey());

    switch (contractName) {
        case 'axelar_gateway': {
            const authAddress = chain.contracts?.axelar_auth_verifiers?.address;

            if (!authAddress) {
                throw new Error('Missing axelar_auth_verifiers contract address');
            }

            return [Address.fromString(authAddress), address];
        }

        case 'axelar_auth_verifiers': {
            const previousSignersRetention = new ScInt(15, { type: 'u64' });
            const domainSeparator = Buffer.alloc(32);
            const miniumumRotationDelay = new ScInt(0, { type: 'u64' });
            const initialSigners = nativeToScVal({
                signers: [
                    {
                        signer: Address.fromString(wallet.publicKey()).toBuffer(),
                        weight: new ScInt(1, { type: 'u128' }),
                    },
                ],
                threshold: new ScInt(1, { type: 'u128' }),
                nonce: Buffer.alloc(32),
            });

            return [address, previousSignersRetention, domainSeparator, miniumumRotationDelay, initialSigners];
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

    let contractAddress = options.address;

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

    const initializeArgs = getInitializeArgs(chain, contractName, wallet, options);
    chain.contracts[contractName].initializeArgs = initializeArgs.map((arg) => arg.toString());

    const contract = new Contract(contractAddress);
    const operation = contract.call('initialize', ...initializeArgs.map((arg) => arg.toScVal()));

    printInfo(
        'Initializing contract with args',
        initializeArgs.map((arg) => arg.toString()),
    );

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

    addEnvOption(program);
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
