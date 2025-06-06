'use strict';

import { 
    Contract, 
    Account, 
    RpcProvider, 
    stark, 
    shortString, 
    CallData, 
    constants, 
    Call,
    InvokeFunctionResponse,
    DeclareContractResponse,
    CompiledContract,
    DeclareContractPayload,
    UniversalDetails,
    InvokeTransactionReceiptResponse,
} from 'starknet';
import * as fs from 'fs';
import * as path from 'path';
import {
    ChainConfig,
    ContractConfig,
    Config,
    DeploymentResult,
    UpgradeResult,
    DeclareResult,
    ContractArtifact,
    UnsignedTransaction,
    GenerateUnsignedTxOptions,
    OfflineTransactionOptions,
    OfflineTransactionResult,
    ResourceBounds
} from './types';

/**
 * Get Starknet provider for the specified chain
 */
export const getStarknetProvider = (chain: ChainConfig): RpcProvider => {
    return new RpcProvider({ nodeUrl: chain.rpc });
};

/**
 * Get Starknet account from private key and address
 */
export const getStarknetAccount = (
    privateKey: string, 
    accountAddress: string, 
    provider: RpcProvider
): Account => {
    return new Account(provider, accountAddress, privateKey, undefined, constants.TRANSACTION_VERSION.V3);
};

/**
 * Deploy a contract to Starknet
 */
export const deployContract = async (
    account: Account, 
    classHash: string, 
    constructorCalldata: any[] = [], 
    salt: string = '0'
): Promise<DeploymentResult> => {
    try {
        const deployResponse = await account.deployContract({
            classHash,
            constructorCalldata,
            salt,
        }, {
            version: '0x3'
        } as UniversalDetails);

        await account.waitForTransaction(deployResponse.transaction_hash);

        return {
            contractAddress: deployResponse.contract_address,
            transactionHash: deployResponse.transaction_hash,
            classHash,
        };
    } catch (error: any) {
        throw new Error(`Contract deployment failed: ${error.message}`);
    }
};

/**
 * Upgrade a contract on Starknet
 */
export const upgradeContract = async (
    account: Account, 
    contractAddress: string, 
    newClassHash: string
): Promise<UpgradeResult> => {
    try {
        // Call upgrade function on the contract
        const upgradeCall: Call = {
            contractAddress,
            entrypoint: 'upgrade',
            calldata: CallData.compile([newClassHash])
        };

        const response = await account.execute(upgradeCall, undefined, {
            version: '0x3'
        } as UniversalDetails);
        await account.waitForTransaction(response.transaction_hash);

        return {
            contractAddress,
            transactionHash: response.transaction_hash,
            newClassHash,
        };
    } catch (error: any) {
        throw new Error(`Contract upgrade failed: ${error.message}`);
    }
};

/**
 * Declare a contract class on Starknet
 */
export const declareContract = async (
    account: Account, 
    contractArtifact: ContractArtifact
): Promise<DeclareResult> => {
    try {
        const declareResponse = await account.declare(contractArtifact as any, {
            version: '0x3'
        } as UniversalDetails);
        await account.waitForTransaction(declareResponse.transaction_hash);

        return {
            classHash: declareResponse.class_hash,
            transactionHash: declareResponse.transaction_hash,
        };
    } catch (error: any) {
        throw new Error(`Contract declaration failed: ${error.message}`);
    }
};

/**
 * Load contract artifact from file
 */
export const loadContractArtifact = (contractName: string): ContractArtifact => {
    const artifactPath = path.join(__dirname, 'artifacts', contractName);

    const sierraPath = path.join(artifactPath, `${contractName}.contract_class.json`);
    const casmPath = path.join(artifactPath, `${contractName}.compiled_contract_class.json`);

    if (!fs.existsSync(sierraPath) || !fs.existsSync(casmPath)) {
        throw new Error(`Contract artifacts not found for ${contractName}. Expected files at ${sierraPath} and ${casmPath}`);
    }

    return {
        contract: JSON.parse(fs.readFileSync(sierraPath, 'utf8')),
        casm: JSON.parse(fs.readFileSync(casmPath, 'utf8')),
    };
};

/**
 * Get contract configuration from chain config
 */
export const getContractConfig = (
    config: Config, 
    chainName: string, 
    contractName: string
): ContractConfig => {
    const chain = config.chains[chainName];
    if (!chain) {
        throw new Error(`Chain ${chainName} not found in configuration`);
    }

    return chain.contracts?.[contractName] || {};
};

/**
 * Save contract deployment info to config
 */
export const saveContractConfig = (
    config: Config, 
    chainName: string, 
    contractName: string, 
    deploymentInfo: Partial<ContractConfig>
): void => {
    if (!config.chains[chainName]) {
        throw new Error(`Chain ${chainName} not found in configuration`);
    }

    if (!config.chains[chainName].contracts) {
        config.chains[chainName].contracts = {};
    }

    config.chains[chainName].contracts![contractName] = {
        ...config.chains[chainName].contracts![contractName],
        ...deploymentInfo,
        deployedAt: new Date().toISOString(),
    };
};

/**
 * Convert string to felt
 */
export const stringToFelt = (str: string): string => {
    return shortString.encodeShortString(str);
};

/**
 * Convert felt to string
 */
export const feltToString = (felt: string): string => {
    return shortString.decodeShortString(felt);
};

/**
 * Generate unsigned transaction for offline signing
 */
export const generateUnsignedTransaction = (
    account: Account | string, 
    calls: Call[], 
    options: GenerateUnsignedTxOptions
): UnsignedTransaction => {
    try {
        const { nonce, resourceBounds } = options;
        if (!nonce) {
            throw new Error('Nonce is required for offline transaction generation');
        }

        const accountAddress = typeof account === 'string' ? account : account.address;

        const unsignedTx: UnsignedTransaction = {
            type: 'INVOKE',
            version: constants.TRANSACTION_VERSION.V3,
            sender_address: accountAddress,
            calls: calls.map(call => ({
                contract_address: call.contractAddress,
                entry_point: call.entrypoint,
                calldata: Array.isArray(call.calldata) ? call.calldata.map(String) : CallData.compile(call.calldata)
            })),
            nonce,
            resource_bounds: resourceBounds,
            tip: '0x0',
            paymaster_data: [],
            account_deployment_data: [],
            nonce_data_availability_mode: 'L1',
            fee_data_availability_mode: 'L1',
            timestamp: Date.now(),
        };

        return unsignedTx;
    } catch (error: any) {
        throw new Error(`Failed to generate unsigned transaction: ${error.message}`);
    }
};

/**
 * Save unsigned transaction to file
 */
export const saveUnsignedTransaction = (
    unsignedTx: UnsignedTransaction, 
    outputDir: string = './starknet-offline-txs', 
    filename?: string
): string => {
    if (!fs.existsSync(outputDir)) {
        fs.mkdirSync(outputDir, { recursive: true });
    }

    // Generate timestamp
    const timestamp = new Date().toISOString().replace(/:/g, '-').replace(/\./g, '-');

    // Handle filename with timestamp
    let finalFilename: string;
    if (filename) {
        // Insert timestamp before the file extension
        const ext = path.extname(filename);
        const baseName = path.basename(filename, ext);
        finalFilename = `${baseName}_${timestamp}${ext}`;
    } else {
        // Default filename with timestamp
        finalFilename = `unsigned_tx_${timestamp}.json`;
    }

    const filepath = path.join(outputDir, finalFilename);

    fs.writeFileSync(filepath, JSON.stringify(unsignedTx, null, 2));
    return filepath;
};

/**
 * Validate Starknet execution options including mainnet security requirements and key management
 */
export const validateStarknetOptions = (
    env: string, 
    offline: boolean, 
    privateKey?: string, 
    accountAddress?: string, 
    requiresTransaction: boolean = true
): void => {
    // Check mainnet offline requirement for transaction operations
    if (requiresTransaction && env === 'mainnet' && !offline) {
        throw new Error('Mainnet environment requires offline flag (--offline) for security. All mainnet transactions must use hardware wallets with offline signing.');
    }

    // Validate key management requirements
    if (requiresTransaction) {
        // Account address is always required for transactions (both online and offline)
        if (!accountAddress) {
            throw new Error('Account address (--accountAddress) is required for transaction operations.');
        }

        // Private key is only required for online execution
        if (!offline && !privateKey) {
            throw new Error('Private key (--privateKey) is required for online transaction execution. Use --offline flag for offline transaction generation.');
        }
    }
};

/**
 * Common handler for offline transaction generation
 */
export const handleOfflineTransaction = (
    options: OfflineTransactionOptions, 
    chainName: string, 
    contractAddress: string, 
    entrypoint: string, 
    calldata: any[], 
    operationName: string
): OfflineTransactionResult => {
    const {
        nonce,
        accountAddress,
        outputDir,
        l1GasMaxAmount = '50000',
        l1GasMaxPricePerUnit = '10000000000',
        l2GasMaxAmount = '5000',
        l2GasMaxPricePerUnit = '1000000000',
    } = options;

    if (!nonce) {
        throw new Error('Nonce is required for offline transaction generation. Use --nonce flag.');
    }
    if (!accountAddress) {
        throw new Error('Account address is required for offline transaction generation. Use --accountAddress flag.');
    }

    console.log(`\nGenerating unsigned transaction for ${operationName} on ${chainName}...`);

    // Create contract call
    const calls: Call[] = [{
        contractAddress,
        entrypoint,
        calldata
    }];

    // Build resource bounds with provided values (defaults applied by CLI)
    const resourceBounds: ResourceBounds = {
        l1_gas: {
            max_amount: '0x' + parseInt(l1GasMaxAmount).toString(16),
            max_price_per_unit: '0x' + parseInt(l1GasMaxPricePerUnit).toString(16)
        },
        l2_gas: {
            max_amount: '0x' + parseInt(l2GasMaxAmount).toString(16),
            max_price_per_unit: '0x' + parseInt(l2GasMaxPricePerUnit).toString(16)
        }
    };

    const unsignedTx = generateUnsignedTransaction(accountAddress, calls, {
        nonce,
        resourceBounds,
    });

    // Save unsigned transaction
    const txFilepath = saveUnsignedTransaction(unsignedTx, outputDir || './starknet-offline-txs',
        `${operationName}_${chainName}.json`);

    console.log(`✅ Unsigned transaction generated successfully!`);
    console.log(`Transaction file: ${txFilepath}`);
    console.log(`\nNext steps:`);
    console.log(`1. Transfer the transaction file to your offline signing environment`);
    console.log(`2. Sign the transaction using your Ledger or signing script`);
    console.log(`3. Broadcast the signed transaction using the broadcast script`);

    return { offline: true, transactionFile: txFilepath };
};