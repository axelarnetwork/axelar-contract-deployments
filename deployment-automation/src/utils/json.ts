/**
 * JSON handling utilities
 */

import * as fs from 'fs';
import { config } from '../config/environment';


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
  
    // Write back the updated JSON
    fs.writeFileSync(networkJsonPath, JSON.stringify(existingJson, null, 2));
    console.log(`✅ Successfully added '${config.CHAIN_NAME}' to ${networkJsonPath}`);
  }