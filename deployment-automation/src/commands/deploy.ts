/**
 * Deployment commands
 */

import * as path from 'path';
import * as fs from 'fs';
import { execSync } from 'child_process';
import { config } from '../config/environment';
import { isCustomDevnet } from '../config/network';
import { downloadContractFiles } from '../contracts/download';
import { deployContracts } from '../contracts/deploy';
import { updateNetworkWithChainConfig } from '../utils/json';
import { setupWallet, getTokenDenomination } from '../wallet/setup';
import { 
  deployGatewayContract, 
  extractProxyGatewayAddress, 
  extractRouterAddress, 
  extractGatewayAddress,
  registerChainWithRouter,
  submitChainRegistrationProposal
} from '../axelar/gateway';
import { 
  updateMultisigProver, 
  retrieveMultisigAddresses, 
  authorizeMultisigProver 
} from '../axelar/multisig';
import { 
  updateVotingVerifierConfig,
  retrieveVotingVerifierAddress 
} from '../axelar/verification';
import { 
  retrieveRewardsAddress 
} from '../axelar/rewards';
import { saveJsonToFile, loadJsonFromFile } from '../utils/fs';
import { displayMessage, MessageType } from '../utils/cli';
import { filterSensitiveData } from '../utils/env';
import { CONFIG_DIR } from '../../constants';

/**
 * Run deployment setup for a new chain
 */
export async function runNewDeployment(userVersion?: string): Promise<void> {
  try {
    // Create entry into namespace json
    updateNetworkWithChainConfig();

    // Extract the predicted gateway proxy address
    try {
      const setupOutput = execSync(`node ../evm/deploy-amplifier-gateway.js --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -m "${config.DEPLOYMENT_TYPE}" --minimumRotationDelay "${config.MINIMUM_ROTATION_DELAY}" --predictOnly -p "${config.TARGET_CHAIN_PRIVATE_KEY}"`, { stdio: 'pipe' }).toString();

      // Print output for debugging
      console.log(setupOutput);

      // Extract the predicted gateway proxy address
      extractProxyGatewayAddress(setupOutput);
    } catch (error) {
      console.error(`Error running deployment script: ${error}`);
      throw error;
    }

    // Call the functions to update JSON
    updateVotingVerifierConfig();
    updateMultisigProver();

    if (isCustomDevnet()) {
      // Setup wallet for custom devnet
      await setupWallet();
      
      // Download contract files and get the paths
      const contractFiles = await downloadContractFiles(userVersion || "latest");
      
      // Deploy contracts using full file paths
      await deployContracts(contractFiles);

      // Get wallet address and token denomination
      await getTokenDenomination();
    } else {
      try {
        await deployContracts(new Map()); // Empty map for non-custom devnet
      } catch (error) {
        console.error(`Error instantiating contracts: ${error}`);
        throw error;
      }
    }

    // Run the functions to extract values
    extractRouterAddress();
    extractGatewayAddress();

    if (isCustomDevnet()) {
      // Run the command to register the chain
      await registerChainWithRouter();
    } else {
      await submitChainRegistrationProposal();
    }

    // Generate extra envs for next steps needed as part of verifier set
    try {
      retrieveRewardsAddress();
      retrieveMultisigAddresses();
      retrieveVotingVerifierAddress();
    } catch (error) {
      console.error(`Error extracting addresses: ${error}`);
      throw error;
    }

    console.log("🎉 Chain registration complete! Need to Update the Verifiers!");

    // Save deployment config for future use
    saveDeploymentConfig();

  } catch (error) {
    displayMessage(MessageType.ERROR, `Deployment failed: ${error}`);
    throw error;
  }
}

/**
 * Save the current deployment configuration to the network's config file
 */
export function saveDeploymentConfig(): void {
    const configKeys = [
      'NAMESPACE', 'CHAIN_NAME', 'CHAIN_ID', 'TOKEN_SYMBOL', 'GAS_LIMIT',
      'RPC_URL', 'AXELAR_RPC_URL',
      'GOVERNANCE_ADDRESS', 'ADMIN_ADDRESS', 'SERVICE_NAME', 'VOTING_THRESHOLD',
      'SIGNING_THRESHOLD', 'CONFIRMATION_HEIGHT', 'MINIMUM_ROTATION_DELAY',
      'DEPLOYMENT_TYPE', 'DEPLOYER', 'CONTRACT_ADMIN', 'PROVER_ADMIN',
      'DEPOSIT_VALUE', 'REWARD_AMOUNT', 'TOKEN_DENOM', 'PROXY_GATEWAY_ADDRESS',
      'ROUTER_ADDRESS', 'GATEWAY_ADDRESS', 'MULTISIG_ADDRESS', 'MULTISIG_PROVER_ADDRESS',
      'VOTING_VERIFIER_ADDRESS', 'REWARDS_ADDRESS', 'COORDINATOR_ADDRESS', 'WALLET_ADDRESS'
    ];
    
    const configData: Record<string, string> = {};
    
    for (const key of configKeys) {
      if (config[key]) {
        configData[key] = config[key]!;
      }
    }
    
    // Filter out sensitive data
    const configToSave = filterSensitiveData(configData);
    
    // The path to the network config file
    const networkConfigPath = path.join(CONFIG_DIR, `${config.NAMESPACE}.json`);
    
    // Load the existing network config
    let networkConfig: any = {};
    if (fs.existsSync(networkConfigPath)) {
      try {
        networkConfig = JSON.parse(fs.readFileSync(networkConfigPath, 'utf8'));
      } catch (error) {
        displayMessage(MessageType.ERROR, `Error loading network config: ${error}`);
        // Fall back to creating a new config
        networkConfig = {};
      }
    }
    
    // Save the current config under the chain name
    if (config.CHAIN_NAME) {
      // Initialize the deployments section if it doesn't exist
      if (!networkConfig.deployments) {
        networkConfig.deployments = {
          default: {
            GOVERNANCE_ADDRESS: networkConfig.axelar?.contracts?.ServiceRegistry?.governanceAccount || "",
            ADMIN_ADDRESS: networkConfig.axelar?.contracts?.ServiceRegistry?.adminAccount || "",
            SERVICE_NAME: "validators",
            VOTING_THRESHOLD: JSON.stringify(["6", "10"]),
            SIGNING_THRESHOLD: JSON.stringify(["6", "10"]),
            CONFIRMATION_HEIGHT: "1",
            MINIMUM_ROTATION_DELAY: "0",
            DEPLOYMENT_TYPE: "create",
            DEPLOYER: "0xba76c6980428A0b10CFC5d8ccb61949677A61233",
            CONTRACT_ADMIN: networkConfig.axelar?.contracts?.ServiceRegistry?.governanceAccount || "",
            PROVER_ADMIN: networkConfig.axelar?.contracts?.ServiceRegistry?.adminAccount || "",
            DEPOSIT_VALUE: "100000000",
            REWARD_AMOUNT: "1000000uamplifier"
          }
        };
      }
      
      networkConfig.deployments[config.CHAIN_NAME] = configToSave;
      
      // Write the updated config back to the file
      fs.writeFileSync(networkConfigPath, JSON.stringify(networkConfig, null, 2));
      
      displayMessage(MessageType.SUCCESS, 
        `Deployment config for ${config.CHAIN_NAME} saved to ${networkConfigPath}. Sensitive data has been excluded.`);
      displayMessage(MessageType.INFO, 
        `Use your original .env file when resuming deployment.`);
    } else {
      displayMessage(MessageType.ERROR, `Cannot save config: CHAIN_NAME is not set.`);
    }
  }