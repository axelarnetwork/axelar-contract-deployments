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
  displayHeader, 
  displayMessage,
  MessageType 
} from './src/utils/cli-utils';
import { loadJsonFromFile } from './src/utils/fs';
import { 
  loadEnvVarsIntoConfig,
  createEnvTemplate
} from './src/utils/env';
import { 
  parseCommandLineArgs,
  printHelp
} from './src/utils/cli-args';
import { 
  runNewDeployment, 
} from './src/commands/deploy';
import { 
  gotoAfterMultisigProposals, 
  printEnvJsonAndExit 
} from './src/commands/resume';
import { CONFIG_DIR } from './constants';

/**
 * Main function to drive the deployment process
 */
async function main(): Promise<void> {
  displayHeader("Axelar Deployment Setup");

  try {
    // Load environment variables into config
    loadEnvVarsIntoConfig(config);
    
    // Parse command-line arguments
    parseCommandLineArgs();
    
    // Check if the .env file exists, if not create a template
    if (!fs.existsSync('.env')) {
      displayMessage(MessageType.WARNING, "No .env file found. Creating a template...");
      createEnvTemplate();
      displayMessage(MessageType.INFO, "Please fill in the required values in the .env file and run the program again.");
      process.exit(0);
    }

    // Check if help was requested
    if (process.argv.includes('--help')) {
      printHelp();
      process.exit(0);
    }

    // Determine if this is a new deployment or continuation
    const isNewDeployment = process.argv.includes('--new-deployment');
    const isResumingDeployment = process.argv.includes('--resume-deployment');

    // Validate that either new or resume flag is provided
    if (!isNewDeployment && !isResumingDeployment) {
      displayMessage(MessageType.ERROR, "Must specify either --new-deployment or --resume-deployment");
      printHelp();
      process.exit(1);
    }

    // Validate that not both flags are provided
    if (isNewDeployment && isResumingDeployment) {
      displayMessage(MessageType.ERROR, "Cannot specify both --new-deployment and --resume-deployment");
      printHelp();
      process.exit(1);
    }

    // Check if deployment is a continuation
    if (isResumingDeployment) {
      // Get chain name from config, error if not provided
      const chainName = config.CHAIN_NAME;
      if (!chainName) {
        displayMessage(MessageType.ERROR, "Chain name must be specified with --chain-name when resuming a deployment");
        process.exit(1);
      }
      
      // Get network name from config, error if not provided
      const namespace = config.NAMESPACE;
      if (!namespace) {
        displayMessage(MessageType.ERROR, "Namespace must be specified with --namespace when resuming a deployment");
        process.exit(1);
      }
      
      const networkConfigPath = path.join(CONFIG_DIR, `${namespace}.json`);
      
      displayMessage(MessageType.INFO, `Loading configuration from ${networkConfigPath}...`);
      
      if (!fs.existsSync(networkConfigPath)) {
        displayMessage(MessageType.ERROR, `Network config file not found: ${networkConfigPath}`);
        process.exit(1);
      }

      // Load the network configuration from JSON
      try {
        const networkConfig = loadJsonFromFile(networkConfigPath);
        
        // Check if deployments section exists
        if (!networkConfig.deployments) {
          displayMessage(MessageType.ERROR, `No deployments found in ${networkConfigPath}`);
          process.exit(1);
        }
        
        // Ensure the chain name exists in deployments
        if (!networkConfig.deployments[chainName]) {
          displayMessage(MessageType.ERROR, `No deployment found for chain '${chainName}' in ${networkConfigPath}`);
          
          // List available deployments
          const availableChains = Object.keys(networkConfig.deployments)
            .filter(key => key !== 'default');
          
          if (availableChains.length > 0) {
            displayMessage(MessageType.INFO, "Available chains:");
            availableChains.forEach(chain => console.log(`- ${chain}`));
          }
          
          process.exit(1);
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

        displayMessage(MessageType.SUCCESS, "Environment restored! Resuming deployment...");

        // Check verifier registration status from CLI args
        const verifiersRegistered = process.argv.includes('--verifiers-registered');
        const verifiersNotRegistered = process.argv.includes('--no-verifiers-registered');

        // Validate that either verifiers-registered or no-verifiers-registered is provided
        if (!verifiersRegistered && !verifiersNotRegistered) {
          displayMessage(MessageType.ERROR, "Must specify either --verifiers-registered or --no-verifiers-registered when resuming");
          process.exit(1);
        }

        // Validate that not both flags are provided
        if (verifiersRegistered && verifiersNotRegistered) {
          displayMessage(MessageType.ERROR, "Cannot specify both --verifiers-registered and --no-verifiers-registered");
          process.exit(1);
        }

        if (verifiersRegistered) {
          // Check proposal approval status from CLI args
          const proposalsApproved = process.argv.includes('--proposals-approved');
          const proposalsNotApproved = process.argv.includes('--no-proposals-approved');

          // Validate that either proposals-approved or no-proposals-approved is provided
          if (!proposalsApproved && !proposalsNotApproved) {
            displayMessage(MessageType.ERROR, "When --verifiers-registered is specified, must also specify either --proposals-approved or --no-proposals-approved");
            process.exit(1);
          }

          // Validate that not both flags are provided
          if (proposalsApproved && proposalsNotApproved) {
            displayMessage(MessageType.ERROR, "Cannot specify both --proposals-approved and --no-proposals-approved");
            process.exit(1);
          }
            
          if (proposalsApproved) {
            await gotoAfterMultisigProposals();
          } else {
            //deprecate
            //await gotoAfterChainRegistration();
          }
        } else {
          printEnvJsonAndExit();
        }
      } catch (error) {
        displayMessage(MessageType.ERROR, `Error loading network configuration: ${error}`);
        process.exit(1);
      }

      return;
    }

    // New deployment flow
    displayMessage(MessageType.INFO, "Starting new deployment using values from .env file");
    
    // Get network from env, error if not provided
    if (!config.NAMESPACE) {
      displayMessage(MessageType.ERROR, "Namespace must be specified with --namespace or in .env for new deployments");
      process.exit(1);
    }
    
    displayMessage(MessageType.INFO, `Using network: ${config.NAMESPACE}`);
    
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
    

    displayMessage(MessageType.INFO, "Environment Variables Set:");
    console.log(`   NETWORK=${config.NAMESPACE}`);
    console.log(`   CHAIN_NAME=${config.CHAIN_NAME}`);
    console.log(`   CHAIN_ID=${config.CHAIN_ID}`);
    console.log(`   TOKEN_SYMBOL=${config.TOKEN_SYMBOL}`);
    console.log(`   GAS_LIMIT=${config.GAS_LIMIT}`);
    console.log(`   RPC_URL=${config.RPC_URL}`);
    console.log(`   AXELAR_RPC_URL=${config.AXELAR_RPC_URL}`);
    console.log(`   GATEWAY_VERSION=${config.GATEWAY_CONTRACT_VERSION}`);
    console.log(`   MULTISIG_PROVER_CONTRACT_VERSION=${config.MULTISIG_PROVER_CONTRACT_VERSION}`);
    console.log(`   VOTING_VERIFIER_VERSION=${config.VOTING_VERIFIER_CONTRACT_VERSION}`);

    // Run the deployment process
    await runNewDeployment();

  } catch (error) {
    displayMessage(MessageType.ERROR, `Deployment failed: ${error}`);
    process.exit(1);
  }
}

// Run the main function
main().catch(error => {
  console.error(`Unhandled error in main: ${error}`);
  process.exit(1);
});