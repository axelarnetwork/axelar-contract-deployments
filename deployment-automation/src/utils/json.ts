/**
 * JSON handling utilities
 */

import * as fs from 'fs';
import * as path from 'path';
import { config } from '../config/environment';
import { saveJsonToFile } from './fs';
import { displayMessage, MessageType } from './cli-utils';
import { CONFIG_DIR } from '../../constants';

/**
 * Function to generate the JSON config file
 */
export function generateJsonConfig(): void {
  const jsonContent = {
    [config.CHAIN_NAME!]: {
      name: config.CHAIN_NAME,
      id: config.CHAIN_ID,
      axelarId: config.CHAIN_NAME,
      chainId: parseInt(config.CHAIN_ID!),
      rpc: config.RPC_URL,
      tokenSymbol: config.TOKEN_SYMBOL,
      confirmations: 1,
      gasOptions: {
        gasLimit: parseInt(config.GAS_LIMIT!)
      }
    }
  };

  // Before saving the temporary config file, check if network config exists
  const networkConfigPath = path.join(CONFIG_DIR, `${config.NAMESPACE}.json`);
  
  // Check if network config already exists
  if (fs.existsSync(networkConfigPath)) {
    try {
      const networkConfig = JSON.parse(fs.readFileSync(networkConfigPath, 'utf8'));
      
      // Initialize deployments section if it doesn't exist
      if (!networkConfig.deployments) {
        networkConfig.deployments = {
          default: config.CHAIN_NAME
        };
        
        // Write the updated network config back to file
        fs.writeFileSync(networkConfigPath, JSON.stringify(networkConfig, null, 2));
        displayMessage(MessageType.INFO, `Added deployments section to ${networkConfigPath}`);
      }
    } catch (error) {
      displayMessage(MessageType.ERROR, `Error updating network config: ${error}`);
    }
  }

  // Save the JSON content to a temporary config file for the current operation
  saveJsonToFile("./config.json", jsonContent);
  console.log(`✅ Temporary configuration saved to ./config.json`);
}

/**
 * Function to insert the generated JSON into the network config file
 */
export function insertIntoNetworkConfig(): void {
  const networkJsonPath = `../axelar-chains-config/info/${config.NAMESPACE}.json`;

  // Check if the network JSON file exists
  if (!fs.existsSync(networkJsonPath)) {
    console.log(`❌ Network JSON file not found: ${networkJsonPath}`);
    throw new Error(`Network JSON file not found: ${networkJsonPath}`);
  }

  // Read the JSON file
  const existingJson = JSON.parse(fs.readFileSync(networkJsonPath, 'utf8'));

  // Check if "chains" exists in the JSON
  if (!existingJson.chains) {
    console.log(`❌ No 'chains' dictionary found in ${networkJsonPath}`);
    throw new Error(`No 'chains' dictionary found in ${networkJsonPath}`);
  }

  // Check if CHAIN_NAME already exists in "chains"
  if (existingJson.chains[config.CHAIN_NAME!]) {
    console.log(`❌ Chain '${config.CHAIN_NAME}' already exists in ${networkJsonPath}! Aborting to prevent overwriting.`);
    throw new Error(`Chain '${config.CHAIN_NAME}' already exists in ${networkJsonPath}! Aborting to prevent overwriting.`);
  }

  // Insert the new chain object into "chains"
  const newChain = JSON.parse(fs.readFileSync('./config.json', 'utf8'));
  existingJson.chains = { ...existingJson.chains, ...newChain };
  
  // Make sure the deployments section exists
  if (!existingJson.deployments) {
    existingJson.deployments = {
      default: config.CHAIN_NAME
    };
  }

  // Write back the updated JSON
  fs.writeFileSync(networkJsonPath, JSON.stringify(existingJson, null, 2));
  console.log(`✅ Successfully added '${config.CHAIN_NAME}' to ${networkJsonPath}`);
}

/**
 * Function to build JSON command for chain registration
 */
export function buildJsonCmdRegister(): string {
  const jsonCmdRegister = JSON.stringify({
    register_chain: {
      chain: config.CHAIN_NAME,
      gateway_address: config.GATEWAY_ADDRESS,
      msg_id_format: "hex_tx_hash_and_event_index"
    }
  });
  
  console.log(`✅ Built JSON_CMD_REGISTER: ${jsonCmdRegister}`);
  return jsonCmdRegister;
}

/**
 * Function to update network configuration with chain info
 */
export function updateNetworkWithChainConfig(): void {
    // Create the chain configuration object directly
    const chainConfig = {
      name: config.CHAIN_NAME,
      id: config.CHAIN_ID,
      axelarId: config.CHAIN_NAME,
      chainId: parseInt(config.CHAIN_ID!),
      rpc: config.RPC_URL,
      tokenSymbol: config.TOKEN_SYMBOL,
      confirmations: 1,
      gasOptions: {
        gasLimit: parseInt(config.GAS_LIMIT!)
      }
    };
  
    // Path to the network JSON file
    const networkJsonPath = `../axelar-chains-config/info/${config.NAMESPACE}.json`;
  
    // Check if the network JSON file exists
    if (!fs.existsSync(networkJsonPath)) {
      console.log(`❌ Network JSON file not found: ${networkJsonPath}`);
      throw new Error(`Network JSON file not found: ${networkJsonPath}`);
    }
  
    // Read the existing JSON file
    const existingJson = JSON.parse(fs.readFileSync(networkJsonPath, 'utf8'));
  
    // Check if "chains" exists in the JSON
    if (!existingJson.chains) {
      console.log(`❌ No 'chains' dictionary found in ${networkJsonPath}`);
      throw new Error(`No 'chains' dictionary found in ${networkJsonPath}`);
    }
  
    // Check if CHAIN_NAME already exists in "chains"
    if (existingJson.chains[config.CHAIN_NAME!]) {
      console.log(`❌ Chain '${config.CHAIN_NAME}' already exists in ${networkJsonPath}! Aborting to prevent overwriting.`);
      throw new Error(`Chain '${config.CHAIN_NAME}' already exists in ${networkJsonPath}! Aborting to prevent overwriting.`);
    }
  
    // Add the chain configuration directly to the chains section
    existingJson.chains[config.CHAIN_NAME!] = chainConfig;
  
    // Initialize deployments section if it doesn't exist
    if (!existingJson.deployments) {
      // Create the deployments section with network default values
      existingJson.deployments = {
        default: {
          GOVERNANCE_ADDRESS: existingJson.axelar?.contracts?.ServiceRegistry?.governanceAccount || "",
          ADMIN_ADDRESS: existingJson.axelar?.contracts?.ServiceRegistry?.adminAccount || "",
          SERVICE_NAME: "validators",
          VOTING_THRESHOLD: JSON.stringify(["6", "10"]),
          SIGNING_THRESHOLD: JSON.stringify(["6", "10"]),
          CONFIRMATION_HEIGHT: "1",
          MINIMUM_ROTATION_DELAY: "0",
          DEPLOYMENT_TYPE: "create",
          DEPLOYER: "0xba76c6980428A0b10CFC5d8ccb61949677A61233",
          CONTRACT_ADMIN: existingJson.axelar?.contracts?.ServiceRegistry?.governanceAccount || "",
          PROVER_ADMIN: "amplifier",
          DEPOSIT_VALUE: "100000000",
          REWARD_AMOUNT: "1000000",

        }
      };
    }
  
    // Write back the updated JSON
    fs.writeFileSync(networkJsonPath, JSON.stringify(existingJson, null, 2));
    console.log(`✅ Successfully added '${config.CHAIN_NAME}' to ${networkJsonPath}`);
  }