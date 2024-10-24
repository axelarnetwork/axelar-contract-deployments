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

async function deploy(options, config, chain) {
    const { contractName, privateKey, wasmPath } = options;
    const { rpc, networkType } = chain;
    const networkPassphrase = getNetworkPassphrase(networkType);
    const wallet = await getWallet(chain, options);

    if (!chain.contracts) {
        chain.contracts = {};
        return;
    }

    const args = `--source ${privateKey} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}"`;
    const cmd = `${stellarCmd} contract deploy --wasm ${wasmPath} ${args}`;

    let contractAddress = options.address;

    if (!contractAddress) {
        contractAddress = execSync(cmd, { encoding: 'utf-8', stdio: 'pipe' }).trimEnd();
        printInfo('Contract deployed successfully!', contractAddress);
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

async function upgrade(options, _, chain) {
    const { contractName, privateKey, wasmPath } = options;
    const { rpc, networkType } = chain;
    const networkPassphrase = getNetworkPassphrase(networkType);
    const contractAddress = chain.contracts?.axelar_gateway.address;

    if (!contractAddress) {
        throw new Error('Stellar Gateway Address not found.');
    }

    const args = `--source ${privateKey} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}"`;

    let cmd = `${stellarCmd} contract install --wasm ${wasmPath} ${args}`;
    const new_wasm_hash = execSync(cmd, { encoding: 'utf-8', stdio: 'pipe' }).trimEnd();

    cmd = `${stellarCmd} contract invoke --id ${contractAddress} ${args} -- upgrade --new_wasm_hash ${new_wasm_hash}`;
    execSync(cmd, { encoding: 'utf-8', stdio: 'pipe' }).trimEnd();

    printInfo('Contract upgraded successfully!', contractAddress);
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    await processor(options, config, getChainConfig(config, options.chainName));
    saveConfig(config, options.env);
}

function main() {
    // 1st level command
    const program = new Command('deploy-contract').description('Deploy/Upgrade Soroban contracts on Stellar');

    // 2nd level deploy command
    const deployCmd = new Command('deploy')
        .description('Deploy a Soroban contract')
        .argument('<contract-name>', 'contract name to deploy')
        .addOption(new Option('--wasm-path <wasmPath>', 'path to the WASM file').makeOptionMandatory(true))
        .addOption(new Option('--nonce <nonce>', 'optional nonce for the signer set'))
        .addOption(new Option('--initialize', 'initialize the contract'))
        .addOption(new Option('--domain-separator <domainSeparator>', 'domain separator (keccak256 hash or "offline")').default('offline'))
        .addOption(
            new Option('--previous-signers-retention <previousSignersRetention>', 'previous signer retention')
                .default(15)
                .argParser(Number),
        )
        .action((contractName, options) => {
            printInfo('Deploying contract', contractName);
            options.contractName = contractName;
            mainProcessor(options, deploy);
        });

    // 2nd level upgrade command
    const upgradeCmd = new Command('upgrade')
        .description('Upgrade a Soroban contract')
        .argument('<contract-name>', 'contract name to deploy')
        .addOption(new Option('--wasm-path <wasmPath>', 'path to the WASM file'))
        .action((contractName, options) => {
            printInfo('Upgrading contract', contractName);
            options.contractName = contractName;
            mainProcessor(options, upgrade);
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
