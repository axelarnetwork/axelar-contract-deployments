#!/usr/bin/env node

'use strict';

const { Command } = require('commander');
const { loadConfig, saveConfig, prompt } = require('../common');
const { addStarknetOptions } = require('./cli-utils');
const {
    deployContract,
    upgradeContract,
    declareContract,
    loadContractArtifact,
    getContractConfig,
    saveContractConfig,
    handleOfflineTransaction,
    validateStarknetOptions,
} = require('./utils');
const { CallData } = require('starknet');

async function processCommand(config, chain, options) {
    const {
        privateKey,
        accountAddress,
        contractName,
        classHash,
        constructorCalldata,
        salt,
        upgrade,
        contractAddress,
        yes,
        offline,
        env,
    } = options;

    // Validate execution options
    validateStarknetOptions(env, offline, privateKey, accountAddress);

    // Handle offline mode
    if (offline) {
        console.log(`\nGenerating unsigned transaction for ${upgrade ? 'upgrading' : 'deploying'} ${contractName} on ${chain.name}...`);

        let targetContractAddress, entrypoint, calldata;

        if (upgrade) {
            if (!contractAddress && !getContractConfig(config, chain.name, contractName).address) {
                throw new Error('Contract address required for upgrade. Provide --contractAddress or ensure contract exists in config.');
            }

            const upgradeTargetAddress = contractAddress || getContractConfig(config, chain.name, contractName).address;

            if (!classHash) {
                throw new Error('Class hash required for upgrade. Provide --classHash.');
            }

            // Prepare upgrade call
            targetContractAddress = upgradeTargetAddress;
            entrypoint = 'upgrade';
            calldata = CallData.compile([classHash]);
        } else {
            // For deployment using Universal Deployer Contract (UDC)
            if (!classHash) {
                throw new Error('Class hash is required for offline deployment. Declare the contract first and provide --classHash.');
            }

            // Get Universal Deployer Address from config
            const universalDeployerAddress = chain.universalDeployerAddress;
            if (!universalDeployerAddress) {
                throw new Error('Universal Deployer Address not found in chain configuration');
            }

            // Parse constructor calldata if provided
            let parsedCalldata = [];
            if (constructorCalldata) {
                try {
                    parsedCalldata = JSON.parse(constructorCalldata);
                } catch (error) {
                    throw new Error(`Invalid constructor calldata JSON: ${error.message}`);
                }
            }

            const deploymentSalt = salt || '0x0';

            targetContractAddress = universalDeployerAddress;
            entrypoint = 'deployContract';
            calldata = CallData.compile([
                classHash,
                deploymentSalt,
                true, // origin dependant deployment
                parsedCalldata,
            ]);
        }

        // Use common offline transaction handler
        const operationName = upgrade ? `upgrade_${contractName}` : `deploy_${contractName}`;
        return handleOfflineTransaction(options, chain.name, targetContractAddress, entrypoint, calldata, operationName);
    }

    console.log(`\n${upgrade ? 'Upgrading' : 'Deploying'} ${contractName} on ${chain.name}...`);

    if (upgrade) {
        if (!contractAddress && !getContractConfig(config, chain.name, contractName).address) {
            throw new Error('Contract address required for upgrade. Provide --contractAddress or ensure contract exists in config.');
        }

        const targetAddress = contractAddress || getContractConfig(config, chain.name, contractName).address;

        if (!classHash) {
            throw new Error('Class hash required for upgrade. Provide --classHash.');
        }

        if (!yes) {
            const confirmUpgrade = prompt(`Are you sure you want to upgrade ${contractName} at ${targetAddress} to class hash ${classHash}?`);
            if (!confirmUpgrade) {
                console.log('Upgrade cancelled.');
                return;
            }
        }

        console.log(`Upgrading contract at ${targetAddress} to class hash ${classHash}...`);
        const upgradeResult = await upgradeContract(account, targetAddress, classHash);

        console.log(`Contract upgraded successfully!`);
        console.log(`Contract Address: ${upgradeResult.contractAddress}`);
        console.log(`Transaction Hash: ${upgradeResult.transactionHash}`);
        console.log(`New Class Hash: ${upgradeResult.newClassHash}`);

        // Update config with new class hash
        saveContractConfig(config, chain.name, contractName, {
            classHash: upgradeResult.newClassHash,
            lastUpgradeTransactionHash: upgradeResult.transactionHash,
        });

    } else {
        // Deploy new contract
        let finalClassHash = classHash;

        if (!finalClassHash) {
            // Need to declare the contract first
            console.log(`Loading contract artifact for ${contractName}...`);
            const contractArtifact = loadContractArtifact(contractName);

            console.log(`Declaring contract ${contractName}...`);
            const declareResult = await declareContract(account, contractArtifact);

            console.log(`Contract declared successfully!`);
            console.log(`Class Hash: ${declareResult.classHash}`);
            console.log(`Transaction Hash: ${declareResult.transactionHash}`);

            finalClassHash = declareResult.classHash;
        }

        // Parse constructor calldata if provided
        let parsedCalldata = [];
        if (constructorCalldata) {
            try {
                parsedCalldata = JSON.parse(constructorCalldata);
            } catch (error) {
                throw new Error(`Invalid constructor calldata JSON: ${error.message}`);
            }
        }

        if (!yes) {
            const confirmDeploy = prompt(`Deploy ${contractName} with class hash ${finalClassHash}?`);
            if (!confirmDeploy) {
                console.log('Deployment cancelled.');
                return;
            }
        }

        console.log(`Deploying contract ${contractName}...`);
        const deployResult = await deployContract(account, finalClassHash, parsedCalldata, salt);

        console.log(`Contract deployed successfully!`);
        console.log(`Contract Address: ${deployResult.contractAddress}`);
        console.log(`Transaction Hash: ${deployResult.transactionHash}`);
        console.log(`Class Hash: ${deployResult.classHash}`);

        // Save deployment info to config
        saveContractConfig(config, chain.name, contractName, {
            address: deployResult.contractAddress,
            classHash: deployResult.classHash,
            deploymentTransactionHash: deployResult.transactionHash,
            deployer: accountAddress,
            salt: salt || '0',
        });
    }

    return config;
}

async function main() {
    const program = new Command();

    program
        .name('deploy-contract')
        .description('Deploy or upgrade Starknet contracts')
        .version('1.0.0');

    addStarknetOptions(program, {
        ignorePrivateKey: true,
        contractName: true,
        classHash: true,
        constructorCalldata: true,
        salt: true,
        upgrade: true,
        contractAddress: true,
        offlineSupport: true,
    });

    program.parse();

    const options = program.opts();
    const { env, chainNames } = options;

    // Validate execution options before processing any chains
    validateStarknetOptions(env, options.offline, options.privateKey, options.accountAddress);

    const config = loadConfig(env);
    const chains = chainNames.split(',').map(name => name.trim());

    for (const chainName of chains) {
        const chain = config.chains[chainName];
        if (!chain) {
            throw new Error(`Chain ${chainName} not found in environment ${env}`);
        }

        if (chain.chainType !== 'starknet') {
            console.log(`Skipping ${chainName} - not a Starknet chain`);
            continue;
        }

        try {
            await processCommand(config, { ...chain, name: chainName }, options);
            console.log(`✅ ${options.upgrade ? 'Upgrade' : 'Deployment'} completed for ${chainName}\n`);
        } catch (error) {
            console.error(`❌ ${options.upgrade ? 'Upgrade' : 'Deployment'} failed for ${chainName}: ${error.message}\n`);
            process.exit(1);
        }
    }

    saveConfig(config, env);
    console.log('Configuration updated successfully.');
}

if (require.main === module) {
    main().catch((error) => {
        console.error('Script failed:', error);
        process.exit(1);
    });
}

module.exports = {
    processCommand,
};
