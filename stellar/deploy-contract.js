'use strict';

const { Address, nativeToScVal, scValToNative } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { execSync, spawnSync } = require('child_process');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { stellarCmd, getNetworkPassphrase, getWallet, serializeValue, addBaseOptions } = require('./utils');
const { getDomainSeparator, getChainConfig } = require('../common');
const { prompt, validateParameters } = require('../common/utils');
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
                domain_separator: domainSeparator,
                minimum_rotation_delay: minimumRotationDelay,
                previous_signers_retention: previousSignersRetention,
                initial_signers: initialSigners,
            };
        }

        case 'interchain_token_service':
            return { owner };
        case 'axelar_operators':
            return { owner };
        case 'axelar_gas_service': {
            const operatorsAddress = chain?.contracts?.axelar_operators?.address;
            const gasCollector = operatorsAddress ? nativeToScVal(Address.fromString(operatorsAddress), { type: 'address' }) : owner;

            return { gas_collector: gasCollector };
        }
        default:
            throw new Error(`Unknown contract: ${contractName}`);
    }
}

async function deploy(options, config, chain, contractName) {
    const { privateKey, wasmPath, yes } = options;
    const { rpc, networkType } = chain;
    const networkPassphrase = getNetworkPassphrase(networkType);
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }
    let contractAddress = options.address;

    if (contractAddress) {
        printInfo('Contract already exists', contractAddress);
        return;
    }

    const initializeArgs = await getInitializeArgs(config, chain, contractName, wallet, options);
    const serializedArgs = Object.fromEntries(
        Object.entries(initializeArgs).map(([key, value]) => [key, serializeValue(scValToNative(value))]),
    );

    printInfo('Initializing contract with args', JSON.stringify(serializedArgs, null, 2));

    // construct arguments in this way to avoid encoding issues when passing arguments to the stellar CLI
    function churn(key, value) {
        if (typeof value === 'object') {
            return [`--${key}`, JSON.stringify(value)];
        } else {
            return [`--${key}`, value];
        }
    }
    const child = spawnSync(
        stellarCmd,
        [
            'contract',
            'deploy',
            '--source',
            `${privateKey}`,
            '--wasm',
            `${wasmPath}`,
            '--rpc-url',
            `${rpc}`,
            '--network-passphrase',
            `${networkPassphrase}`,
            '--',
        ].concat(
            Object.entries(serializedArgs)
                .map(([key, value]) => churn(key, value))
                .flat(1),
        ),
    );
    contractAddress = String(child.stdout);
    printInfo('Contract deployed successfully!', contractAddress);

    chain.contracts[contractName] = {
        address: contractAddress,
        deployer: wallet.publicKey(),
    };
}

async function upgrade(options, _, chain, contractName) {
    const { privateKey, wasmPath, yes } = options;
    const { rpc, networkType } = chain;
    const networkPassphrase = getNetworkPassphrase(networkType);
    const contractAddress = chain.contracts[contractName].address;

    if (prompt(`Proceed with upgrade on ${chain.name}?`, yes)) {
        return;
    }

    validateParameters({
        isNonEmptyString: { contractAddress },
    });

    const params = `--source ${privateKey} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}"`;

    let cmd = `${stellarCmd} contract install --wasm ${wasmPath} ${params}`;
    const newWasmHash = execSync(cmd, { encoding: 'utf-8', stdio: 'pipe' }).trimEnd();

    printInfo('New Wasm hash', newWasmHash);

    cmd = `${stellarCmd} contract invoke --id ${contractAddress} ${params} -- upgrade --new_wasm_hash ${newWasmHash}`;
    execSync(cmd, { encoding: 'utf-8', stdio: 'pipe' }).trimEnd();

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
