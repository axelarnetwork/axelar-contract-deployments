'use strict';

import {
    CompiledContract,
    Call
} from 'starknet';

/**
 * Resource bounds for Starknet transactions (L1 and L2 gas limits)
 */
export interface ResourceBounds {
    l1_gas: {
        max_amount: string;
        max_price_per_unit: string;
    };
    l2_gas: {
        max_amount: string;
        max_price_per_unit: string;
    };
    l1_data: {
        max_amount: string;
        max_price_per_unit: string;
    };
}

/**
 * Configuration for a blockchain network chain
 * Contains RPC endpoints and contract deployment information
 */
export interface ChainConfig {
    /** RPC endpoint URL for connecting to the chain */
    rpc: string;
    /** Optional mapping of contract names to their deployment configurations */
    contracts?: Record<string, ContractConfig>;
    /** Universal deployer contract address for Starknet deployments */
    universalDeployerAddress?: string;
    /** Chain name identifier */
    name?: string;
}

/**
 * Configuration for a deployed contract
 * Stores deployment metadata and addresses
 */
export interface ContractConfig {
    /** On-chain address of the deployed contract */
    address?: string;
    /** Class hash of the contract implementation */
    classHash?: string;
    /** Transaction hash from the initial deployment */
    deploymentTransactionHash?: string;
    /** Transaction hash from the most recent upgrade */
    lastUpgradeTransactionHash?: string;
    /** Address of the account that deployed the contract */
    deployer?: string;
    /** Salt used for deterministic deployment */
    salt?: string;
    /** ISO timestamp of when the contract was deployed */
    deployedAt?: string;
}

/**
 * Root configuration object containing all chain configurations
 */
export interface Config {
    /** Mapping of chain names to their configurations */
    chains: Record<string, ChainConfig>;
}

/**
 * Result returned after successful contract deployment
 */
export interface DeploymentResult {
    /** Address where the contract was deployed */
    contractAddress: string;
    /** Transaction hash of the deployment */
    transactionHash: string;
    /** Class hash of the deployed contract */
    classHash: string;
}

/**
 * Result returned after successful contract upgrade
 */
export interface UpgradeResult {
    /** Address of the upgraded contract */
    contractAddress: string;
    /** Transaction hash of the upgrade transaction */
    transactionHash: string;
    /** New class hash after upgrade */
    newClassHash: string;
}

/**
 * Result returned after successful contract declaration
 */
export interface DeclareResult {
    /** Class hash of the declared contract */
    classHash: string;
    /** Transaction hash of the declaration */
    transactionHash: string;
}

/**
 * Contract artifact containing compiled contract data
 * Used for declaring contracts on Starknet
 */
export interface ContractArtifact {
    /** Sierra compiled contract (high-level representation) */
    contract: CompiledContract;
    /** CASM compiled contract (Cairo assembly) */
    casm: CompiledContract;
}

/**
 * Base fields for all unsigned transactions
 */
interface BaseUnsignedTransaction {
    /** Transaction type (e.g., 'INVOKE', 'DECLARE', 'DEPLOY') */
    type: string;
    /** Transaction version (e.g., '0x3' for v3 transactions) */
    version: string;
    /** Address of the account sending the transaction */
    sender_address: string;
    /** Account nonce for transaction ordering */
    nonce: string;
    /** Gas limits and pricing for L1 and L2 */
    resource_bounds: ResourceBounds;
    /** Optional tip for block producers */
    tip: string;
    /** Data for paymaster sponsorship (if applicable) */
    paymaster_data: any[];
    /** Data availability mode for nonce (L1 or L2) */
    nonce_data_availability_mode: string;
    /** Data availability mode for fee (L1 or L2) */
    fee_data_availability_mode: string;
    /** Unix timestamp of transaction creation */
    timestamp: number;
}

/**
 * Unsigned invoke transaction for offline signing
 */
export interface UnsignedInvokeTransaction extends BaseUnsignedTransaction {
    type: 'INVOKE';
    /** Array of contract calls to execute */
    calls: Array<{
        /** Target contract address */
        contract_address: string;
        /** Function to call on the contract */
        entry_point: string;
        /** Encoded function arguments */
        calldata: string[];
    }>;
    /** Data for account deployment (if applicable) */
    account_deployment_data: any[];
}

/**
 * Unsigned declare transaction for offline signing
 */
export interface UnsignedDeclareTransaction extends BaseUnsignedTransaction {
    type: 'DECLARE';
    /** Contract class to declare */
    contract_class: CompiledContract;
    /** Compiled class hash */
    compiled_class_hash: string;
}

/**
 * Union type for all unsigned transactions
 */
export type UnsignedTransaction = UnsignedInvokeTransaction | UnsignedDeclareTransaction;

/**
 * Options for generating unsigned transactions
 */
export interface GenerateUnsignedTxOptions {
    /** Account nonce for the transaction */
    nonce: string;
    /** Gas limits and pricing configuration */
    resourceBounds: ResourceBounds;
}

/**
 * Options for offline transaction generation
 * Used when creating transactions for hardware wallet signing
 */
export interface OfflineTransactionOptions {
    /** Current account nonce */
    nonce?: string;
    /** Account address that will sign the transaction */
    accountAddress?: string;
    /** Directory to save unsigned transaction files */
    outputDir?: string;
    /** Maximum L1 gas amount */
    l1GasMaxAmount?: string;
    /** Maximum L1 gas price per unit */
    l1GasMaxPricePerUnit?: string;
    /** Maximum L2 gas amount */
    l2GasMaxAmount?: string;
    /** Maximum L2 gas price per unit */
    l2GasMaxPricePerUnit?: string;
    /** Maximum L1 data amount */
    l1DataMaxAmount?: string;
    /** Maximum L1 data price per unit */
    l1DataMaxPricePerUnit?: string;
    /** Compiled class hash (required for declare transactions, computed with starkli) */
    compiledClassHash?: string;
    /** Whether offline mode is enabled */
    offline?: boolean;
}

/**
 * Result of offline transaction generation
 */
export interface OfflineTransactionResult {
    /** Indicates this was an offline operation */
    offline: boolean;
    /** Path to the saved transaction file */
    transactionFile: string;
}

/**
 * Base options for CLI commands
 * Common options available across all commands
 */
export interface BaseCommandOptions {
    /** Environment name (mainnet, testnet, devnet, stagenet) */
    env: string;
    /** Comma-separated list of chain names to operate on */
    chainNames: string;
    /** Skip confirmation prompts */
    yes?: boolean;
}

/**
 * Options specific to Starknet operations
 * Extends base options with Starknet-specific parameters
 */
export interface StarknetCommandOptions extends BaseCommandOptions, OfflineTransactionOptions {
    /** Private key for transaction signing (testnet/devnet only) */
    privateKey?: string;
    /** Whether to add options for a specific feature */
    ignorePrivateKey?: boolean;
    /** Whether to ignore account address requirement */
    ignoreAccountAddress?: boolean;
}

/**
 * Options for contract deployment commands
 */
export interface DeployContractOptions extends StarknetCommandOptions {
    /** Name of the contract to deploy */
    contractName?: string;
    /** Pre-declared class hash (skips declaration step) */
    classHash?: string;
    /** JSON-encoded constructor arguments */
    constructorCalldata?: string;
    /** Salt for deterministic deployment addresses */
    salt?: string;
    /** Whether this is an upgrade operation */
    upgrade?: boolean;
    /** Contract address for upgrade operations */
    contractAddress?: string;
}

/**
 * Options for contract upgrade commands
 */
export interface UpgradeContractOptions extends StarknetCommandOptions {
    /** Name of the contract to upgrade */
    contractName?: string;
    /** New class hash for the upgrade */
    classHash?: string;
    /** Contract address to upgrade (optional if already in config) */
    contractAddress?: string;
}

/**
 * Options for gateway contract interactions
 */
export interface GatewayCommandOptions extends StarknetCommandOptions {
    /** Destination chain for cross-chain calls */
    destinationChain?: string;
    /** Destination contract address */
    destinationContractAddress?: string;
    /** Payload data to send */
    payload?: string;
    /** Source chain for message validation */
    sourceChain?: string;
    /** Message identifier */
    messageId?: string;
    /** Source address for validation */
    sourceAddress?: string;
    /** Hash of the payload */
    payloadHash?: string;
    /** New operator address for transfers */
    newOperator?: string;
    /** Messages for approval */
    messages?: any[];
    /** Proof data for verification */
    proof?: any;
    /** New signers configuration */
    newSigners?: any;
}

/**
 * Configuration for CLI command options
 * Used to dynamically add command-line flags
 */
export interface CliOptionConfig {
    /** Skip private key option */
    ignorePrivateKey?: boolean;
    /** Skip account address option */
    ignoreAccountAddress?: boolean;
    /** Is the command related to contract declaration? */
    declaration?: boolean;
    /** Is the command related to contract deployment? */
    deployment?: boolean;
    /** Is the command related to upgrading a deployment? */
    upgrade?: boolean;
    /** Add contract address option */
    contractAddress?: boolean;
    /** Enable offline transaction support */
    offlineSupport?: boolean;
}

