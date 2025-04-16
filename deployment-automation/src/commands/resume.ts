/**
 * Continuation/resume commands
 */

import { config } from '../config/environment';
import { verifyExecution, retrieveVotingVerifierAddress } from '../axelar/verification';
import { 
  retrieveMultisigAddresses, 
  verifyMultisig, 
  authorizeMultisigProver,
  createGenesisVerifierSet,
  registerMultisigProverWithCoordinator 
} from '../axelar/multisig';
import { 
  retrieveRewardsAddress, 
  addFundsToRewardPools,
  createMultisigRewardPool,
  createVotingVerifierRewardPool 
} from '../axelar/rewards';
import { deployGatewayContract, submitChainRegistrationProposal } from '../axelar/gateway';
import { saveDeploymentConfig } from './deploy';
import { displayMessage, MessageType } from '../utils/cli-utils';


export async function gotoResubmitProposals(): Promise<void> {
    displayMessage(MessageType.INFO, "Resubmit Proposals...");
  
    try {
        // Generate extra envs for next steps needed as part of verifier set
        try {
          retrieveRewardsAddress();
          retrieveMultisigAddresses();
          retrieveVotingVerifierAddress();
        } catch (error) {
          console.error(`Error extracting addresses: ${error}`);
          throw error;
        }

        // Capture the proposal ID returned from submitChainRegistrationProposal
        const registerChainProposalId = await submitChainRegistrationProposal();
        if (registerChainProposalId) {
          console.log(`✅ Chain Gateway registration proposal submitted with ID: ${registerChainProposalId}`);
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

      // Explicitly exit the process
      process.exit(0);
    } catch (error) {
      displayMessage(MessageType.ERROR, `Chain registration resume failed: ${error}`);
      process.exit(1);
    }
  }
/**
 * Function to handle the state after multisig proposals have been approved
 */
export async function gotoAfterMultisigProposals(): Promise<void> {
  try {
    // Run the verification step that gateway router was registered
    await verifyExecution();

    // Verify multisig
    await verifyMultisig();

    // Try to add funds to reward pools, but continue on failure
    try {
      await addFundsToRewardPools();
    } catch (error) {
      displayMessage(MessageType.WARNING, `Adding funds to reward pools encountered an issue: ${error}`);
      displayMessage(MessageType.INFO, "Continuing with deployment...");
    }

    // Try to create genesis verifier set, but continue on failure
    try {
      await createGenesisVerifierSet();
    } catch (error) {
      // Check if the error is because the verifier set hasn't changed
      const errorStr = String(error);
      if (errorStr.includes("verifier set has not changed sufficiently since last update")) {
        displayMessage(MessageType.WARNING, "Verifier set has not changed sufficiently. This is normal if it was recently updated.");
      } else {
        displayMessage(MessageType.WARNING, `Creating genesis verifier set encountered an issue: ${error}`);
        displayMessage(MessageType.INFO, "Continuing with deployment...");
      }
    }

    // Deploy gateway contract (this is the critical step)
    try {
      const gatewayOutput = await deployGatewayContract();
      console.log(gatewayOutput);
      displayMessage(MessageType.SUCCESS, "Gateway deployed successfully!");
    } catch (error) {
      displayMessage(MessageType.ERROR, `Gateway deployment failed: ${error}`);
      
      // If the --continue-on-error flag is set, exit with a special code
      if (process.argv.includes('--continue-on-error')) {
        displayMessage(MessageType.WARNING, "Continuing despite gateway deployment failure due to --continue-on-error flag");
        process.exit(2); // Special exit code for this case
      } else {
        throw error;
      }
    }

    displayMessage(MessageType.SUCCESS, "🎉 Deployment complete!");
    process.exit(0);
  } catch (error) {
    displayMessage(MessageType.ERROR, `Post-multisig proposals execution failed: ${error}`);
    
    // If the --force-gateway-deployment flag is set, try gateway deployment anyway
    if (process.argv.includes('--force-gateway-deployment')) {
      displayMessage(MessageType.WARNING, "Attempting gateway deployment despite errors due to --force-gateway-deployment flag");
      try {
        const gatewayOutput = await deployGatewayContract();
        console.log(gatewayOutput);
        displayMessage(MessageType.SUCCESS, "Gateway deployment completed!");
      } catch (finalError) {
        displayMessage(MessageType.ERROR, `Final gateway deployment failed: ${finalError}`);
        throw finalError;
      }
    } else {
      throw error;
    }
  }
}

/**
 * Function to print environment variables as JSON and exit
 */
export function printEnvJsonAndExit(): void {
  displayMessage(MessageType.SUCCESS, "Chain registration complete! Need to Update the Verifiers!");
  
  // Save deployment config
  saveDeploymentConfig();
  
  displayMessage(MessageType.INFO, "To continue once verifiers have registered support, run:");
  console.log(`npm start -- --resume-deployment --chain-name ${config.CHAIN_NAME} --verifiers-registered --no-proposals-approved`);
  
  process.exit(0);
}