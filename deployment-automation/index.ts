/**
 * Main entry point for the Axelar deployment process
 */

import * as fs from 'fs';
import * as path from 'path';
import { config } from './src/config/environment';
import { getNetworkName, handleCustomDevnet } from './src/config/network';
import { 
  validatePrivateKey, 
  validateRpcUrl, 
  validateAxelarRpcUrl,
  validateMnemonic,
  validateChainInfo
} from './src/config/validation';
import { 
  question, 
  closeReadline, 
  displayHeader, 
  displayMessage,
  MessageType 
} from './src/utils/cli';
import { loadJsonFromFile } from './src/utils/fs';
import { 
  loadEnvVarsIntoConfig,
  createEnvTemplate
} from './src/utils/env';
import { 
  runNewDeployment, 
  saveDeploymentConfig 
} from './src/commands/deploy';
import { 
  gotoAfterChainRegistration, 
  gotoAfterMultisigProposals, 
  printEnvJsonAndExit 
} from './src/commands/resume';
import { CONFIG_DIR } from './constants';

/**
 * Main function to drive the deployment process
 */
async function main(): Promise<void> {
  displayHeader("Welcome to Axelar Deployment Setup");

  try {
    // Load environment variables into config
    loadEnvVarsIntoConfig(config);
    
    // Check if the .env file exists, if not create a template
    if (!fs.existsSync('.env')) {
      displayMessage(MessageType.WARNING, "No .env file found. Creating a template...");
      createEnvTemplate();
      displayMessage(MessageType.INFO, "Please fill in the required values in the .env file and run the program again.");
      return;
    }

    // Ask user if this is a new deployment or continuation
    const deploymentType = await question("Is this a new deployment? (yes/no): ");

    // Check if deployment is a continuation
    if (deploymentType.toLowerCase() === "no") {
      // Ask for chain name to find the correct config file
      let chainName = config.CHAIN_NAME;
      if (!chainName) {
        chainName = await question("Enter Chain Name for the deployment you want to continue: ");
      }
      
      // Determine the network name
      let namespace = config.NAMESPACE;
      if (!namespace) {
        namespace = await question("Enter the Network namespace (e.g., mainnet, testnet, devnet-myname): ");
      }
      
      const networkConfigPath = path.join(CONFIG_DIR, `${namespace}.json`);
      
      console.log(`✅ Loading configuration from ${networkConfigPath}...`);
      
      if (!fs.existsSync(networkConfigPath)) {
        console.log(`❌ Error: ${networkConfigPath} not found. Cannot resume deployment.`);
        process.exit(1);
      }

      // Load the network configuration from JSON
      try {
        const networkConfig = loadJsonFromFile(networkConfigPath);
        
        // Check if deployments section exists
        if (!networkConfig.deployments) {
          console.log(`❌ No deployments found in ${networkConfigPath}.`);
          process.exit(1);
        }
        
        // Ensure we have a valid chain name
        if (!chainName || !networkConfig.deployments[chainName]) {
          console.log(`❌ No deployment found for chain '${chainName}' in ${networkConfigPath}.`);
          
          // List available deployments
          const availableChains = Object.keys(networkConfig.deployments)
            .filter(key => key !== 'default');
          
          if (availableChains.length > 0) {
            console.log("Available chains:");
            availableChains.forEach(chain => console.log(`- ${chain}`));
            
            // Ask user to select a chain
            chainName = await question("Enter one of the available chain names: ");
            
            if (!networkConfig.deployments[chainName]) {
              console.log(`❌ Invalid chain selection. Exiting.`);
              process.exit(1);
            }
          } else {
            console.log(`❌ No deployments available in ${networkConfigPath}. Exiting.`);
            process.exit(1);
          }
        }
        
        // If there's a default config, use it as a base and override with chain-specific values
        if (networkConfig.deployments.default) {
          // First apply default values
          Object.assign(config, networkConfig.deployments.default);
        }
        
        // Then apply chain-specific values
        const savedConfig = networkConfig.deployments[chainName];
        Object.assign(config, savedConfig);
        
        // Ensure namespace and chain name are set
        config.NAMESPACE = namespace;
        config.CHAIN_NAME = chainName;

        console.log("✅ Environment restored! Resuming deployment...");

        const verifiersRegistered = await question("Have verifiers registered support for the chain? (yes/no): ");

        if (verifiersRegistered.toLowerCase() === "yes") {
          const multisigProposalsApproved = await question("Have multisig proposals been approved? (yes/no): ");
          if (multisigProposalsApproved.toLowerCase() === "yes") {
            await gotoAfterMultisigProposals();
          } else {
            await gotoAfterChainRegistration();
          }
        } else {
          printEnvJsonAndExit();
        }
      } catch (error) {
        console.error(`Error loading network configuration: ${error}`);
        process.exit(1);
      }

      closeReadline();
      return;
    }

    // New deployment flow
    displayMessage(MessageType.INFO, "Starting new deployment using values from .env file");
    
    // Get network from prompt or env
    if (!config.NAMESPACE) {
      await getNetworkName();
    } else {
      displayMessage(MessageType.INFO, `Using network: ${config.NAMESPACE}`);
    }
    
    // Check if default configuration exists in network config and load it
    const networkConfigPath = path.join(CONFIG_DIR, `${config.NAMESPACE}.json`);
    if (fs.existsSync(networkConfigPath)) {
      try {
        const networkConfig = loadJsonFromFile(networkConfigPath);
        if (networkConfig.deployments?.default) {
          // Apply default network values where not already set in environment
          const defaultConfig = networkConfig.deployments.default;
          for (const [key, value] of Object.entries(defaultConfig)) {
            if (!config[key]) {
              config[key] = value as string;
              displayMessage(MessageType.INFO, `Loaded default value for ${key} from network config`);
            }
          }
        }
      } catch (error) {
        displayMessage(MessageType.WARNING, `Could not load default network configuration: ${error}`);
      }
    }
    
    // Validate required chain information
    validateChainInfo();
    
    // Validate sensitive data from environment variables
    validatePrivateKey();
    validateRpcUrl();
    validateAxelarRpcUrl();
    validateMnemonic();
    
    // Set default values for custom devnets
    handleCustomDevnet();
    
    // Ask for contract version
    const userVersion = await question("Enter version to retrieve: ");

    displayMessage(MessageType.INFO, "Environment Variables Set:");
    console.log(`   NETWORK=${config.NAMESPACE}`);
    console.log(`   CHAIN_NAME=${config.CHAIN_NAME}`);
    console.log(`   CHAIN_ID=${config.CHAIN_ID}`);
    console.log(`   TOKEN_SYMBOL=${config.TOKEN_SYMBOL}`);
    console.log(`   GAS_LIMIT=${config.GAS_LIMIT}`);
    console.log(`   RPC_URL=${config.RPC_URL}`);
    console.log(`   AXELAR_RPC_URL=${config.AXELAR_RPC_URL}`);

    // Run the deployment process
    await runNewDeployment(userVersion);

  } catch (error) {
    displayMessage(MessageType.ERROR, `Deployment failed: ${error}`);
    process.exit(1);
  } finally {
    closeReadline();
  }
}

// Run the main function
main().catch(error => {
  console.error(`Unhandled error in main: ${error}`);
  process.exit(1);
});