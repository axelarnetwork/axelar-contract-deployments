/**
 * Environment variable handling utilities
 */

import * as dotenv from 'dotenv';
import * as fs from 'fs';
import * as path from 'path';
import { displayMessage, MessageType } from './cli-utils';

// Load environment variables from .env file
dotenv.config();

/**
 * Sensitive environment variable keys that should not be logged or saved in config files
 */
export const SENSITIVE_ENV_KEYS = [
  'PRIVATE_KEY',
  'MNEMONIC',
  'SECRET_KEY',
  'PASSWORD'
];

/**
 * Map of environment variable aliases
 * Keys are the primary names, values are the aliases
 */
export const ENV_VAR_ALIASES: Record<string, string[]> = {
  // No aliases needed as we're standardizing on PRIVATE_KEY
};

/**
 * Get an environment variable with an optional fallback value
 */
export function getEnvVar(key: string, fallback?: string): string | undefined {
  return process.env[key] || fallback;
}

/**
 * Get a required environment variable
 * Throws an error if the variable is not set and no fallback is provided
 */
export function getRequiredEnvVar(key: string, fallback?: string): string {
  const value = getEnvVar(key, fallback);
  if (value === undefined) {
    throw new Error(`Required environment variable ${key} is not set`);
  }
  return value;
}

/**
 * Check if all required environment variables are set
 */
export function checkRequiredEnvVars(requiredKeys: string[]): boolean {
  const missingKeys: string[] = [];
  
  for (const key of requiredKeys) {
    if (getEnvVar(key) === undefined) {
      missingKeys.push(key);
    }
  }
  
  if (missingKeys.length > 0) {
    displayMessage(
      MessageType.ERROR, 
      `Missing required environment variables: ${missingKeys.join(', ')}\n` +
      `Please add them to your .env file.`
    );
    return false;
  }
  
  return true;
}

/**
 * Create a template .env file if it doesn't exist
 */
export function createEnvTemplate(): void {
  const envPath = path.resolve(process.cwd(), '.env');
  
  if (fs.existsSync(envPath)) {
    displayMessage(MessageType.INFO, '.env file already exists');
    return;
  }
  
  const template = `# Axelar Deployment Environment Variables
# Chain Configuration
CHAIN_NAME=
CHAIN_ID=
TOKEN_SYMBOL=
GAS_LIMIT=

# Network RPC URLs
RPC_URL=
AXELAR_RPC_URL=

# Environment Namespace (used to load config file)
ENV_NAMESPACE=

# Sensitive Data (keep secure!)
PRIVATE_KEY=
MNEMONIC=

# Optional Configuration
NAMESPACE=
GOVERNANCE_ADDRESS=
ADMIN_ADDRESS=
SERVICE_NAME=
VOTING_THRESHOLD=
SIGNING_THRESHOLD=
CONFIRMATION_HEIGHT=
MINIMUM_ROTATION_DELAY=
DEPLOYMENT_TYPE=
`;
  
  fs.writeFileSync(envPath, template);
  displayMessage(MessageType.SUCCESS, '.env template file created. Please fill in the required values before running the deployment.');
}

/**
 * Load configuration from a JSON file based on environment namespace
 */
export function loadConfigFromFile(envNamespace: string): Record<string, any> {
  try {
    const configPath = path.resolve(process.cwd(), `configs/${envNamespace}.json`);
    
    if (!fs.existsSync(configPath)) {
      displayMessage(
        MessageType.WARNING, 
        `Configuration file for environment ${envNamespace} not found: ${configPath}`
      );
      return {};
    }

    const configContent = fs.readFileSync(configPath, 'utf8');
    const config = JSON.parse(configContent);
    
    // Extract default deployment values
    if (config.deployments && config.deployments.default) {
      displayMessage(
        MessageType.INFO, 
        `Loaded configuration for environment: ${envNamespace}`
      );
      return config.deployments.default;
    } else {
      displayMessage(
        MessageType.WARNING, 
        `No default deployment configuration found in ${configPath}`
      );
      return {};
    }
  } catch (error) {
    displayMessage(
      MessageType.ERROR, 
      `Failed to load configuration file: ${error instanceof Error ? error.message : String(error)}`
    );
    return {};
  }
}

/**
 * Load environment variables into the config
 */
export function loadEnvVarsIntoConfig(config: any): void {
  // Check if we have an environment namespace specified
  const envNamespace = getEnvVar('ENV_NAMESPACE');
  
  // If we have an environment namespace, load config from the corresponding file
  if (envNamespace) {
    const fileConfig = loadConfigFromFile(envNamespace);
    
    // Merge file config into the main config
    Object.assign(config, fileConfig);
    displayMessage(
      MessageType.INFO, 
      `Loaded configuration from configs/${envNamespace}.json`
    );
  }
  
  // List of environment variables to check
  const envVarKeys = [
    'NAMESPACE',
    'CHAIN_NAME',
    'CHAIN_ID',
    'TOKEN_SYMBOL',
    'GAS_LIMIT',
    'PRIVATE_KEY',
    'RPC_URL',
    'AXELAR_RPC_URL',
    'MNEMONIC',
    'GOVERNANCE_ADDRESS',
    'ADMIN_ADDRESS',
    'SERVICE_NAME',
    'VOTING_THRESHOLD',
    'SIGNING_THRESHOLD',
    'CONFIRMATION_HEIGHT',
    'MINIMUM_ROTATION_DELAY',
    'DEPLOYMENT_TYPE',
    'DEPLOYER',
    'CONTRACT_ADMIN',
    'PROVER_ADMIN',
    'VOTING_VERIFIER_CONTRACT_VERSION',
    'GATEWAY_CONTRACT_VERSION',
    'MULTISIG_PROVER_CONTRACT_VERSION',
    'RUN_AS_ACCOUNT',
    'EPOCH_DURATION',
    'WALLET_ADDRESS',
    'TOKEN_DENOM',
    'DEPOSIT_VALUE',
    'REWARD_AMOUNT',
    'CHAIN_FINALITY',
    'CHAIN_CONFIRMATIONS',
    'CHAIN_APPROX_FINALITY_WAIT_TIME',
    'AMPD_FINALITY',
    'AXELAR_MULTISIG_ADDRESS'
  ];
  
  // Environment variables take precedence over file config
  for (const key of envVarKeys) {
    const value = getEnvVar(key);
    if (value !== undefined) {
      config[key] = value;
      
      // Only log non-sensitive values
      if (!SENSITIVE_ENV_KEYS.includes(key)) {
        displayMessage(MessageType.INFO, `Loaded ${key} from environment: ${value}`);
      } else {
        displayMessage(MessageType.INFO, `Loaded ${key} from environment`);
      }
    }
  }
}

/**
 * Filter out sensitive data from an object
 */
export function filterSensitiveData<T extends Record<string, any>>(data: T): Partial<T> {
  const filtered = { ...data };
  
  for (const key of SENSITIVE_ENV_KEYS) {
    if (key in filtered) {
      delete filtered[key];
    }
  }
  
  return filtered;
}