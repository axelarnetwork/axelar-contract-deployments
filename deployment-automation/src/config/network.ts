/**
 * Network configuration handling
 */

import * as fs from 'fs';
import * as path from 'path';
import { CONFIG_DIR } from '../../constants';
import { config, EnvironmentConfig } from './environment';
import { question } from '../utils/cli-utils';

/**
 * Function to load network configuration from JSON file
 */
export function loadNetworkConfig(network: string): void {
  const configPath = path.join(CONFIG_DIR, `${network}.json`);
  
  if (!fs.existsSync(configPath)) {
    console.log(`❌ Network configuration file not found: ${configPath}`);
    return;
  }
  
  try {
    const networkConfig = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    
    // Merge the loaded configuration with our config object
    for (const [key, value] of Object.entries(networkConfig)) {
      // Handle arrays (like thresholds) that need to be stringified
      if (Array.isArray(value)) {
        config[key] = JSON.stringify(value);
      } else {
        config[key] = value as string;
      }
    }
    
    console.log(`✅ Loaded network configuration for ${network}`);
  } catch (error) {
    console.error(`❌ Error loading network configuration: ${error}`);
  }
}

/**
 * Function to get the network name from user input
 */
export async function getNetworkName(): Promise<void> {
  console.log("Select a network:");
  console.log("1) mainnet");
  console.log("2) testnet");
  console.log("3) stagenet");
  console.log("4) devnet-amplifier");
  console.log("5) Custom devnet (e.g., devnet-user)");
  
  while (true) {
    const choice = await question("Enter your choice (1-5): ");
    switch (choice) {
      case '1': 
        config.NAMESPACE = "mainnet"; 
        // Load network configuration
        loadNetworkConfig("mainnet");
        return;
      case '2': 
        config.NAMESPACE = "testnet"; 
        // Load network configuration
        loadNetworkConfig("testnet");
        return;
      case '3': 
        config.NAMESPACE = "stagenet"; 
        // Load network configuration
        loadNetworkConfig("stagenet");
        return;
      case '4': 
        config.NAMESPACE = "devnet-amplifier"; 
        // Load network configuration
        loadNetworkConfig("devnet-amplifier");
        return;
      case '5': 
        const customName = await question("Enter your custom devnet name (e.g., devnet-user): ");
        config.NAMESPACE = customName;
        
        // Check if configuration exists for this custom network
        if (fs.existsSync(path.join(CONFIG_DIR, `${customName}.json`))) {
          loadNetworkConfig(customName);
        } else {
          console.log(`⚠️ No configuration found for ${customName}. Using default values.`);
        }
        return;
      default: 
        console.log("❌ Invalid choice. Please select 1, 2, 3, 4 or 5.");
        break;
    }
  }
}

/**
 * Function to check if the network is a custom devnet
 */
export function isCustomDevnet(): boolean {
  if (config.NAMESPACE === "mainnet" || 
      config.NAMESPACE === "testnet" || 
      config.NAMESPACE === "stagenet" || 
      config.NAMESPACE === "devnet-markus") {
    return false; // Not a custom devnet
  } else {
    return true; // Is a custom devnet
  }
}

/**
 * Function to check if the network is a custom devnet and set default values if needed
 */
export function handleCustomDevnet(): void {
    if (isCustomDevnet()) {
      console.log("🔧 Custom devnet detected. Setting default values...");
      
      // Set default values for custom devnet if they're not already set
      if (!config.DEPLOYMENT_TYPE) {
        config.DEPLOYMENT_TYPE = "create";
        console.log(`✅ Set default DEPLOYMENT_TYPE: ${config.DEPLOYMENT_TYPE}`);
      }
      
      if (!config.MINIMUM_ROTATION_DELAY) {
        config.MINIMUM_ROTATION_DELAY = "0";
        console.log(`✅ Set default MINIMUM_ROTATION_DELAY: ${config.MINIMUM_ROTATION_DELAY}`);
      }
      
      // Other default values that might be needed
      if (!config.SERVICE_NAME) {
        config.SERVICE_NAME = "validators";
        console.log(`✅ Set default SERVICE_NAME: ${config.SERVICE_NAME}`);
      }
      
      if (!config.VOTING_THRESHOLD) {
        config.VOTING_THRESHOLD = '["6", "10"]';
        console.log(`✅ Set default VOTING_THRESHOLD: ${config.VOTING_THRESHOLD}`);
      }
      
      if (!config.SIGNING_THRESHOLD) {
        config.SIGNING_THRESHOLD = '["6", "10"]';
        console.log(`✅ Set default SIGNING_THRESHOLD: ${config.SIGNING_THRESHOLD}`);
      }
      
      if (!config.CONFIRMATION_HEIGHT) {
        config.CONFIRMATION_HEIGHT = "1";
        console.log(`✅ Set default CONFIRMATION_HEIGHT: ${config.CONFIRMATION_HEIGHT}`);
      }
  
      // Get governance address from the JSON file
      try {
        const networkJsonPath = `../axelar-chains-config/info/${config.NAMESPACE}.json`;
        
        if (!fs.existsSync(networkJsonPath)) {
          throw new Error(`Network JSON file not found: ${networkJsonPath}`);
        }
        
        // Read the JSON file directly instead of using jq
        const jsonData = JSON.parse(fs.readFileSync(networkJsonPath, 'utf8'));
        
        // Try to get the governance address from ServiceRegistry
        let governanceAddress = jsonData?.axelar?.contracts?.ServiceRegistry?.governanceAccount;
        
        if (!governanceAddress) {
          console.log("⚠️ No ServiceRegistry.governanceAccount found, checking for validators...");
          
          // Try to get the first validator address as a fallback
          if (jsonData.validators && jsonData.validators.length > 0 && jsonData.validators[0].axelarAddress) {
            governanceAddress = jsonData.validators[0].axelarAddress;
            console.log(`⚠️ Using first validator address: ${governanceAddress}`);
          } else {
            // Use address from wallet if available
            if (config.WALLET_ADDRESS) {
              governanceAddress = config.WALLET_ADDRESS;
              console.log(`⚠️ Using wallet address: ${governanceAddress}`);
            } else {
              throw new Error("Could not find a valid governance address");
            }
          }
        }
        
        // Set all admin related fields to the governance address
        config.GOVERNANCE_ADDRESS = governanceAddress;
        config.ADMIN_ADDRESS = governanceAddress;
        config.CONTRACT_ADMIN = governanceAddress;
        config.PROVER_ADMIN = governanceAddress;
        config.DEPLOYER = governanceAddress;
        
        console.log(`✅ Set admin addresses to: ${governanceAddress}`);
      } catch (error) {
        console.error(`❌ Error setting admin addresses: ${error}`);
        console.log("⚠️ Will try to determine admin address during contract deployment");
      }
    }
  }