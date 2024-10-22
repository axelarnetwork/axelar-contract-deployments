'use strict';

const { Contract, Address, nativeToScVal, scValToNative } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { execSync } = require('child_process');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { stellarCmd, getNetworkPassphrase, getWallet, broadcast, serializeValue, addBaseOptions } = require('./utils');
const { getDomainSeparator, getChainConfig } = require('../common');
const { weightedSignersToScVal } = require('./type-utils');
const { ethers } = require('hardhat');
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

        case 'axelar_operators':
            return { operator };
        default:
            throw new Error(`Unknown contract: ${contractName}`);
    }
}

async function processCommand(options, config, chain) {
    const { contractName, contractId, privateKey, wasmPath, newWasmHash } = options;
    const { rpc, networkType } = chain;
    const networkPassphrase = getNetworkPassphrase(networkType);
    const wallet = await getWallet(chain, options);

    if (!chain.contracts) {
        chain.contracts = {};
    }

    const args = `--source ${privateKey} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}"`;

    let cmd;

    if (options.install) {
        cmd = `${stellarCmd} contract install --wasm ${wasmPath} ${args}`;
    } else if (options.upgrade) {
        cmd = `${stellarCmd} contract invoke --id ${contractId} ${args} -- upgrade --new_wasm_hash ${newWasmHash}`;
    } else {
        cmd = `${stellarCmd} contract deploy --wasm ${wasmPath} ${args}`;
    }

    printInfo('Executing command', cmd);
    printInfo('Deploying contract', contractName);

    let contractAddress = options.address;

    if (!contractAddress) {
        const result = execSync(cmd, { encoding: 'utf-8', stdio: 'pipe' }).trimEnd();

        if (options.install) {
            printInfo('Contract WASM hash', result);
            return;
        } else if (options.upgrade) {
            printInfo('Upgraded contract successfully!');
            return;
        }

        printInfo('Deployed contract successfully!', result);
        contractAddress = result;
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

    const initializeArgs = await getInitializeArgs(config, chain, contractName, wallet, options);
    const serializedArgs = Object.fromEntries(
        Object.entries(initializeArgs).map(([key, value]) => [key, serializeValue(scValToNative(value))]),
    );
    chain.contracts[contractName].initializeArgs = serializedArgs;

    const contract = new Contract(contractAddress);
    const operation = contract.call('initialize', ...Object.values(initializeArgs));

    printInfo('Initializing contract with args', JSON.stringify(serializedArgs, null, 2));

    await broadcast(operation, wallet, chain, 'Initialized contract', options);
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    await processor(options, config, getChainConfig(config, options.chainName));
    saveConfig(config, options.env);
}

function main() {
    const program = new Command();
    program.name('deploy-contract').description('Deploy Axelar Soroban contracts on Stellar');

    addBaseOptions(program, { address: true });

    program.addOption(new Option('--initialize', 'initialize the contract'));
    program.addOption(new Option('--contract-name <contractName>', 'contract name to deploy'));
    program.addOption(new Option('--wasm-path <wasmPath>', 'path to the WASM file'));
    program.addOption(new Option('--nonce <nonce>', 'optional nonce for the signer set'));
    program.addOption(new Option('--install', 'install only'));
    program.addOption(new Option('--upgrade', 'upgrade only'));
    program.addOption(new Option('--contract-id <contractId>', 'contract id (address) to upgrade'));
    program.addOption(new Option('--new-wasm-hash <newWasmHash>', 'new WASM hash to upgrade'));
    program.addOption(
        new Option('--previous-signers-retention <previousSignersRetention>', 'previous signer retention').default(15).argParser(Number),
    );

    program.addOption(
        new Option(
            '--domain-separator <domainSeparator>',
            'domain separator (pass in the keccak256 hash value OR "offline" meaning that its computed locally)',
        ).default('offline'),
    );

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}

if (require.main === module) {
    main();
}
