/**
 * Deployment commands
 */

import * as path from 'path';
import * as fs from 'fs';
import { execSync } from 'child_process';
import { config } from '../config/environment';
import { isCustomDevnet } from '../config/network';
import { 
  createVotingVerifierRewardPool, 
  createMultisigRewardPool 
} from '../axelar/rewards';
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
  authorizeMultisigProver,
  registerMultisigProverWithCoordinator 
} from '../axelar/multisig';
import { 
  updateVotingVerifierConfig,
  retrieveVotingVerifierAddress 
} from '../axelar/verification';
import { 
  retrieveRewardsAddress 
} from '../axelar/rewards';
import { saveJsonToFile, loadJsonFromFile } from '../utils/fs';
import { displayMessage, MessageType } from '../utils/cli-utils';
import { filterSensitiveData } from '../utils/env';
import { CONFIG_DIR } from '../../constants';

/**
 * Run deployment setup for a new chain
 */
export async function runNewDeployment(): Promise<void> {
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
      
      // Deploy contracts using full file paths
      await deployContracts();
      
      // Get wallet address and token denomination
      await getTokenDenomination();
    } else {
      try {
        await deployContracts(); // Empty map for non-custom devnet
      } catch (error) {
        console.error(`Error instantiating contracts: ${error}`);
        throw error;
      }
    }
    
    // Run the functions to extract values
    extractRouterAddress();
    extractGatewayAddress();
    
    // Store the proposal ID when submitting chain registration
    if (isCustomDevnet()) {
      // Run the command to register the chain
      await registerChainWithRouter();
    } else {
      // Capture the proposal ID returned from submitChainRegistrationProposal
      const registerChainProposalId = await submitChainRegistrationProposal();
      if (registerChainProposalId) {
        console.log(`✅ Chain Gateway registration proposal submitted with ID: ${registerChainProposalId}`);
      }
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
  
    // Register and authorize MultisigProver
    const coordinatorProposalId = await registerMultisigProverWithCoordinator();
    if (coordinatorProposalId) {
      console.log(`✅ Multisig prover registration proposal submitted with ID: ${coordinatorProposalId}`);
    }
      
    // Authorize multisig prover
    const multisigProposalId = await authorizeMultisigProver();
    if (multisigProposalId) {
      console.log(`✅ Multisig prover authorization proposal submitted with ID: ${multisigProposalId}`);
    }

    // create reward pools
    const multisigRewardProposalId = await createMultisigRewardPool()
    if (multisigRewardProposalId) {
      console.log(`✅ Multisig reward pool proposal submitted with ID: ${multisigRewardProposalId}`);
    }

    const votingVerifierRewardProposalId = await createVotingVerifierRewardPool()
    if (votingVerifierRewardProposalId) {
      console.log(`✅ Multisig reward pool proposal submitted with ID: ${votingVerifierRewardProposalId}`);
    }


    console.log("🎉 Chain registration complete! Need to Update the Verifiers!");
    
    // Save deployment config for future use
    saveDeploymentConfig();
    process.exit(0);
  } catch (error) {
    displayMessage(MessageType.ERROR, `Deployment failed: ${error}`);
    throw error;
  }
}

// 2. Update the saveDeploymentConfig function to include PROPOSAL_ID
export function saveDeploymentConfig(): void {
  const configKeys = [
    'NAMESPACE', 'CHAIN_NAME', 'CHAIN_ID', 'TOKEN_SYMBOL', 'GAS_LIMIT',
    'RPC_URL', 'AXELAR_RPC_URL',
    'GOVERNANCE_ADDRESS', 'ADMIN_ADDRESS', 'SERVICE_NAME', 'VOTING_THRESHOLD',
    'SIGNING_THRESHOLD', 'CONFIRMATION_HEIGHT', 'MINIMUM_ROTATION_DELAY',
    'DEPLOYMENT_TYPE', 'DEPLOYER', 'CONTRACT_ADMIN', 'PROVER_ADMIN',
    'DEPOSIT_VALUE', 'REWARD_AMOUNT', 'TOKEN_DENOM', 'PROXY_GATEWAY_ADDRESS',
    'ROUTER_ADDRESS', 'GATEWAY_ADDRESS', 'MULTISIG_ADDRESS', 'MULTISIG_PROVER_ADDRESS',
    'VOTING_VERIFIER_ADDRESS', 'REWARDS_ADDRESS', 'COORDINATOR_ADDRESS', 'WALLET_ADDRESS',
    'REGISTER_GATEWAY_PROPOSAL_ID', "REGISTER_MULTISIG_PROVER_COORDINATOR_PROPOSAL_ID",
    "AUTHORIZE_MULTISIG_PROVER_PROPOSAL_ID", 'CREATE_VOTING_VERIFIER_REWARD_POOL_PROPOSAL_ID',
    'CREATE_MULTISIG_REWARD_POOL_PROPOSAL_ID'
  ];
  
  const configData: Record<string, string> = {};
  
  for (const key of configKeys) {
    if (config[key] !== undefined) { // Only include keys that have values
      if (typeof config[key] === 'boolean') {
        configData[key] = String(config[key]);
      } else {
        configData[key] = config[key] as string;
      }
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
    
    // Display additional info for the proposal if available
    if (config.PROPOSAL_ID) {
      displayMessage(MessageType.SUCCESS, 
        `Chain registration proposal submitted with ID: ${config.PROPOSAL_ID}`);
    }
    
    displayMessage(MessageType.INFO, 
      `Use your original .env file when resuming deployment.`);
    displayMessage(MessageType.INFO, "Once proposals are approved, rerun with --resume-deployment --chain-name " + 
        config.CHAIN_NAME + " --verifiers-registered --proposals-approved");
  } else {
    displayMessage(MessageType.ERROR, `Cannot save config: CHAIN_NAME is not set.`);
    process.exit(1);
  }
}