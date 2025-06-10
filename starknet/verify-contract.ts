#!/usr/bin/env ts-node

'use strict';

import { Command, Option } from 'commander';
import { loadConfig, printInfo, printError, printWarn, prompt } from '../common';
import { addStarknetOptions } from './cli-utils';
import {
    getContractConfig,
    getStarknetProvider
} from './utils';
const axios = require('axios');
import fs from 'fs';
import path from 'path';
import {
    Config,
    ChainConfig,
    VerifyContractOptions,
    VerificationResult
} from './types';

// Explorer API endpoints
const EXPLORER_APIS = {
    voyager: {
        testnet: 'https://api-sepolia.voyager.online/beta',
        mainnet: 'https://api.voyager.online/beta'
    },
    starkscan: {
        testnet: 'https://api-sepolia.starkscan.co/api',
        mainnet: 'https://api.starkscan.co/api'
    }
};

/**
 * Get the appropriate API endpoint for the explorer and environment
 */
function getExplorerApiUrl(explorer: string, env: string): string {
    const apis = EXPLORER_APIS[explorer.toLowerCase()];
    if (!apis) {
        throw new Error(`Unsupported explorer: ${explorer}. Supported: voyager, starkscan`);
    }

    const isMainnet = env === 'mainnet';
    return isMainnet ? apis.mainnet : apis.testnet;
}

/**
 * Verify contract on Voyager explorer
 */
async function verifyOnVoyager(
    apiUrl: string,
    contractAddress: string,
    classHash: string,
    constructorCalldata: string[],
    sourceFiles: Record<string, string>
): Promise<VerificationResult> {
    try {
        // Voyager verification endpoint
        const verifyUrl = `${apiUrl}/contracts/verify`;

        const payload = {
            contract_address: contractAddress,
            class_hash: classHash,
            constructor_calldata: constructorCalldata,
            source_code: sourceFiles
        };

        printInfo('Submitting verification to Voyager...');
        const response = await axios.post(verifyUrl, payload, {
            headers: {
                'Content-Type': 'application/json'
            }
        });

        if (response.data.status === 'verified' || response.data.verified) {
            return {
                success: true,
                contractAddress,
                explorer: 'voyager',
                message: 'Contract verified successfully on Voyager',
                verificationUrl: `https://sepolia.voyager.online/contract/${contractAddress}`
            };
        } else {
            return {
                success: false,
                contractAddress,
                explorer: 'voyager',
                message: response.data.message || 'Verification failed'
            };
        }
    } catch (error: any) {
        if (error.response?.data?.message) {
            throw new Error(`Voyager API error: ${error.response.data.message}`);
        }
        throw error;
    }
}

/**
 * Verify contract on Starkscan explorer
 */
async function verifyOnStarkscan(
    apiUrl: string,
    contractAddress: string,
    classHash: string,
    constructorCalldata: string[],
    sourceFiles: Record<string, string>
): Promise<VerificationResult> {
    try {
        // Starkscan uses a different API structure
        const verifyUrl = `${apiUrl}/v1/contract/verify`;

        const payload = {
            address: contractAddress,
            class_hash: classHash,
            constructor_args: constructorCalldata,
            sources: sourceFiles,
            compiler_version: 'cairo-2.0.0', // You may need to make this configurable
            contract_name: Object.keys(sourceFiles)[0].replace('.cairo', '')
        };

        printInfo('Submitting verification to Starkscan...');
        const response = await axios.post(verifyUrl, payload, {
            headers: {
                'Content-Type': 'application/json'
            }
        });

        if (response.data.status === 'SUCCESS') {
            return {
                success: true,
                contractAddress,
                explorer: 'starkscan',
                message: 'Contract verified successfully on Starkscan',
                verificationUrl: `https://sepolia.starkscan.co/contract/${contractAddress}`
            };
        } else {
            return {
                success: false,
                contractAddress,
                explorer: 'starkscan',
                message: response.data.message || 'Verification failed'
            };
        }
    } catch (error: any) {
        if (error.response?.data?.message) {
            throw new Error(`Starkscan API error: ${error.response.data.message}`);
        }
        throw error;
    }
}

/**
 * Load source files from a directory
 */
function loadSourceFiles(sourceDir: string): Record<string, string> {
    const sourceFiles: Record<string, string> = {};

    if (!fs.existsSync(sourceDir)) {
        throw new Error(`Source directory not found: ${sourceDir}`);
    }

    // Read all .cairo files in the directory
    const files = fs.readdirSync(sourceDir);
    for (const file of files) {
        if (file.endsWith('.cairo')) {
            const filePath = path.join(sourceDir, file);
            const content = fs.readFileSync(filePath, 'utf-8');
            sourceFiles[file] = content;
        }
    }

    if (Object.keys(sourceFiles).length === 0) {
        throw new Error(`No Cairo source files found in ${sourceDir}`);
    }

    return sourceFiles;
}

async function processCommand(
    config: Config,
    chain: ChainConfig & { name: string },
    options: VerifyContractOptions
): Promise<VerificationResult> {
    const {
        contractConfigName,
        contractAddress,
        explorer = 'voyager',
        sourceDir,
        yes,
        env
    } = options;

    // Get contract info from config or command line
    let address = contractAddress;
    let classHash: string | undefined;
    let constructorCalldata: string[] = [];

    if (contractConfigName) {
        const contractConfig = getContractConfig(config, chain.name, contractConfigName);
        address = address || contractConfig.address;
        classHash = contractConfig.classHash;

        // Try to retrieve constructor calldata from config if stored
        // This is a simplified approach - you might need to enhance this
        // based on how you store constructor arguments
    }

    if (!address) {
        throw new Error('Contract address required. Provide --contractAddress or ensure contract exists in config.');
    }

    if (!classHash) {
        // If class hash not in config, we need to fetch it from the chain
        printInfo('Fetching contract class hash from chain...');
        const provider = getStarknetProvider(chain);
        const contractClass = await provider.getClassHashAt(address);
        classHash = contractClass;
    }

    if (!sourceDir) {
        throw new Error('Source directory required. Provide --sourceDir with path to Cairo source files.');
    }

    // Load source files
    printInfo(`Loading source files from ${sourceDir}...`);
    const sourceFiles = loadSourceFiles(sourceDir);
    printInfo(`Found ${Object.keys(sourceFiles).length} source files`);

    // Get explorer API URL
    const apiUrl = getExplorerApiUrl(explorer, env);

    if (!yes) {
        const shouldCancel = prompt(`Verify contract ${address} on ${explorer}?`);
        if (shouldCancel) {
            throw new Error('Verification cancelled by user');
        }
    }

    // Submit verification based on explorer
    let result: VerificationResult;

    switch (explorer.toLowerCase()) {
        case 'voyager':
            result = await verifyOnVoyager(apiUrl, address, classHash, constructorCalldata, sourceFiles);
            break;
        case 'starkscan':
            result = await verifyOnStarkscan(apiUrl, address, classHash, constructorCalldata, sourceFiles);
            break;
        default:
            throw new Error(`Unsupported explorer: ${explorer}`);
    }

    if (result.success) {
        printInfo(' ' + result.message);
        if (result.verificationUrl) {
            printInfo(`View verified contract: ${result.verificationUrl}`);
        }

        // Optionally update config to mark as verified
        if (contractConfigName) {
            // You could add a 'verified' field to the contract config
            // saveContractConfig(config, chain.name, contractConfigName, { verified: true });
        }
    } else {
        printError('L ' + result.message);
    }

    return result;
}

async function main(): Promise<void> {
    const program = new Command();

    program
        .name('verify-contract')
        .description('Verify Starknet contracts on block explorers')
        .version('1.0.0');

    addStarknetOptions(program, {
        ignorePrivateKey: true,
        ignoreAccountAddress: true
    });

    // Add verification-specific options
    program.addOption(
        new Option('--contractConfigName <contractConfigName>', 'name of the contract configuration to verify')
    );
    program.addOption(
        new Option('--contractAddress <contractAddress>', 'contract address to verify')
    );
    program.addOption(
        new Option('--explorer <explorer>', 'explorer to use for verification')
            .choices(['voyager', 'starkscan'])
            .default('voyager')
    );
    program.addOption(
        new Option('--sourceDir <sourceDir>', 'directory containing Cairo source files')
            .makeOptionMandatory(true)
    );

    program.parse();

    const options = program.opts() as VerifyContractOptions;
    const { env } = options;

    const config = loadConfig(env);
    const chainName = 'starknet';
    const chain = config.chains[chainName];

    if (!chain) {
        throw new Error(`Chain ${chainName} not found in environment ${env}`);
    }

    // Validate that we have either contractConfigName or contractAddress
    if (!options.contractConfigName && !options.contractAddress) {
        throw new Error('Either --contractConfigName or --contractAddress must be provided');
    }

    try {
        await processCommand(config, { ...chain, name: chainName }, options);
        printInfo('Verification process completed');
    } catch (error: any) {
        printError(`Verification failed: ${error.message}`);
        process.exit(1);
    }
}

if (require.main === module) {
    main().catch((error) => {
        console.error('Script failed:', error);
        process.exit(1);
    });
}

export {
    processCommand,
    verifyOnVoyager,
    verifyOnStarkscan
};

