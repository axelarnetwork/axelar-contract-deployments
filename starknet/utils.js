'use strict';

const { Contract, Account, RpcProvider, stark, shortString, CallData, constants } = require('starknet');
const fs = require('fs');
const path = require('path');

/**
 * Get Starknet provider for the specified chain
 * @param {Object} chain - Chain configuration
 * @returns {RpcProvider} Starknet RPC provider
 */
const getStarknetProvider = (chain) => {
    return new RpcProvider({ nodeUrl: chain.rpc });
};

/**
 * Get Starknet account from private key and address
 * @param {string} privateKey - Private key
 * @param {string} accountAddress - Account address
 * @param {RpcProvider} provider - Starknet provider
 * @returns {Account} Starknet account
 */
const getStarknetAccount = (privateKey, accountAddress, provider) => {
    return new Account(provider, accountAddress, privateKey, undefined, constants.TRANSACTION_VERSION.V3);
};

/**
 * Deploy a contract to Starknet
 * @param {Account} account - Starknet account
 * @param {string} classHash - Contract class hash
 * @param {Array} constructorCalldata - Constructor calldata
 * @param {string} salt - Salt for deployment
 * @returns {Object} Deployment result
 */
const deployContract = async (account, classHash, constructorCalldata = [], salt = '0') => {
    try {
        const deployResponse = await account.deployContract({
            classHash,
            constructorCalldata,
            salt,
        }, {
            version: '0x3'
        });

        await account.waitForTransaction(deployResponse.transaction_hash);

        return {
            contractAddress: deployResponse.contract_address,
            transactionHash: deployResponse.transaction_hash,
            classHash,
        };
    } catch (error) {
        throw new Error(`Contract deployment failed: ${error.message}`);
    }
};

/**
 * Upgrade a contract on Starknet
 * @param {Account} account - Starknet account
 * @param {string} contractAddress - Contract address to upgrade
 * @param {string} newClassHash - New class hash
 * @returns {Object} Upgrade result
 */
const upgradeContract = async (account, contractAddress, newClassHash) => {
    try {
        // Call upgrade function on the contract
        const upgradeCall = {
            contractAddress,
            entrypoint: 'upgrade',
            calldata: CallData.compile([newClassHash])
        };

        const response = await account.execute(upgradeCall, undefined, {
            version: '0x3'
        });
        await account.waitForTransaction(response.transaction_hash);

        return {
            contractAddress,
            transactionHash: response.transaction_hash,
            newClassHash,
        };
    } catch (error) {
        throw new Error(`Contract upgrade failed: ${error.message}`);
    }
};

/**
 * Declare a contract class on Starknet
 * @param {Account} account - Starknet account
 * @param {Object} contractArtifact - Contract artifact (sierra and casm)
 * @returns {Object} Declaration result
 */
const declareContract = async (account, contractArtifact) => {
    try {
        const declareResponse = await account.declare(contractArtifact, {
            version: '0x3'
        });
        await account.waitForTransaction(declareResponse.transaction_hash);

        return {
            classHash: declareResponse.class_hash,
            transactionHash: declareResponse.transaction_hash,
        };
    } catch (error) {
        throw new Error(`Contract declaration failed: ${error.message}`);
    }
};

/**
 * Load contract artifact from file
 * @param {string} contractName - Contract name
 * @returns {Object} Contract artifact
 */
const loadContractArtifact = (contractName) => {
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
 * @param {Object} config - Chain configuration
 * @param {string} chainName - Chain name
 * @param {string} contractName - Contract name
 * @returns {Object} Contract configuration
 */
const getContractConfig = (config, chainName, contractName) => {
    const chain = config.chains[chainName];
    if (!chain) {
        throw new Error(`Chain ${chainName} not found in configuration`);
    }

    return chain.contracts?.[contractName] || {};
};

/**
 * Save contract deployment info to config
 * @param {Object} config - Chain configuration
 * @param {string} chainName - Chain name
 * @param {string} contractName - Contract name
 * @param {Object} deploymentInfo - Deployment information
 */
const saveContractConfig = (config, chainName, contractName, deploymentInfo) => {
    if (!config.chains[chainName]) {
        throw new Error(`Chain ${chainName} not found in configuration`);
    }

    if (!config.chains[chainName].contracts) {
        config.chains[chainName].contracts = {};
    }

    config.chains[chainName].contracts[contractName] = {
        ...config.chains[chainName].contracts[contractName],
        ...deploymentInfo,
        deployedAt: new Date().toISOString(),
    };
};

/**
 * Convert string to felt
 * @param {string} str - String to convert
 * @returns {string} Felt representation
 */
const stringToFelt = (str) => {
    return shortString.encodeShortString(str);
};

/**
 * Convert felt to string
 * @param {string} felt - Felt to convert
 * @returns {string} String representation
 */
const feltToString = (felt) => {
    return shortString.decodeShortString(felt);
};

/**
 * Generate unsigned transaction for offline signing
 * @param {Account} account - Starknet account (address only, no network calls)
 * @param {Array} calls - Transaction calls
 * @param {Object} options - Transaction options (must include nonce for offline)
 * @returns {Object} Unsigned transaction data
 */
const generateUnsignedTransaction = (account, calls, options = {}) => {
    try {
        const { nonce, resourceBounds } = options;
        if (!nonce) {
            throw new Error('Nonce is required for offline transaction generation');
        }

        const unsignedTx = {
            type: 'INVOKE',
            version: constants.TRANSACTION_VERSION.V3,
            sender_address: account.address,
            calls: calls.map(call => ({
                contract_address: call.contractAddress,
                entry_point: call.entrypoint,
                calldata: Array.isArray(call.calldata) ? call.calldata : CallData.compile(call.calldata)
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
    } catch (error) {
        throw new Error(`Failed to generate unsigned transaction: ${error.message}`);
    }
};

/**
 * Save unsigned transaction to file
 * @param {Object} unsignedTx - Unsigned transaction
 * @param {string} outputDir - Output directory
 * @param {string} filename - Filename (optional)
 * @returns {string} File path
 */
const saveUnsignedTransaction = (unsignedTx, outputDir = './starknet-offline-txs', filename) => {
    if (!fs.existsSync(outputDir)) {
        fs.mkdirSync(outputDir, { recursive: true });
    }

    // Generate timestamp
    const timestamp = new Date().toISOString().replace(/:/g, '-').replace(/\./g, '-');

    // Handle filename with timestamp
    let finalFilename;
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
 * Check if mainnet environment requires offline flag
 * @param {string} env - Environment name
 * @param {boolean} offline - Whether offline flag is set
 */
const checkMainnetOfflineRequirement = (env, offline) => {
    if (env === 'mainnet' && !offline) {
        throw new Error('Mainnet environment requires offline flag (--offline) for security. All mainnet transactions must use hardware wallets with offline signing.');
    }
};

/**
 * Common handler for offline transaction generation
 * @param {Object} options - Command options containing offline parameters
 * @param {string} chainName - Chain name
 * @param {string} contractAddress - Contract address
 * @param {string} entrypoint - Contract entrypoint
 * @param {Array} calldata - Compiled calldata
 * @param {string} operationName - Operation name for logging and filename
 * @returns {Object} Result with offline flag and transaction file path
 */
const handleOfflineTransaction = (options, chainName, contractAddress, entrypoint, calldata, operationName) => {
    const {
        nonce,
        accountAddress,
        outputDir,
        l1GasMaxAmount,
        l1GasMaxPricePerUnit,
        l2GasMaxAmount,
        l2GasMaxPricePerUnit,
    } = options;

    if (!nonce) {
        throw new Error('Nonce is required for offline transaction generation. Use --nonce flag.');
    }
    if (!accountAddress) {
        throw new Error('Account address is required for offline transaction generation. Use --accountAddress flag.');
    }

    console.log(`\nGenerating unsigned transaction for ${operationName} on ${chainName}...`);

    // Create offline account object (address only, no private key needed)
    const offlineAccount = { address: accountAddress };

    // Create contract call
    const calls = [{
        contractAddress,
        entrypoint,
        calldata
    }];

    // Build resource bounds with provided values (defaults applied by CLI)
    const resourceBounds = {
        l1_gas: {
            max_amount: '0x' + parseInt(l1GasMaxAmount).toString(16),
            max_price_per_unit: '0x' + parseInt(l1GasMaxPricePerUnit).toString(16)
        },
        l2_gas: {
            max_amount: '0x' + parseInt(l2GasMaxAmount).toString(16),
            max_price_per_unit: '0x' + parseInt(l2GasMaxPricePerUnit).toString(16)
        }
    };

    const unsignedTx = generateUnsignedTransaction(offlineAccount, calls, {
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

module.exports = {
    getStarknetProvider,
    getStarknetAccount,
    deployContract,
    upgradeContract,
    declareContract,
    loadContractArtifact,
    getContractConfig,
    saveContractConfig,
    stringToFelt,
    feltToString,
    generateUnsignedTransaction,
    saveUnsignedTransaction,
    handleOfflineTransaction,
    checkMainnetOfflineRequirement,
};
