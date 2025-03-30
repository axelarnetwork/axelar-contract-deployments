/**
 * Continuation/resume commands
 */

import { config } from '../config/environment';
import { verifyExecution } from '../axelar/verification';
import { 
  retrieveMultisigAddresses, 
  verifyMultisig, 
  authorizeMultisigProver,
  createGenesisVerifierSet 
} from '../axelar/multisig';
import { 
  createRewardPools, 
  addFundsToRewardPools 
} from '../axelar/rewards';
import { deployGatewayContract } from '../axelar/gateway';
import { saveDeploymentConfig } from './deploy';
import { displayMessage, MessageType } from '../utils/cli';

/**
 * This is the continuation point if the script is resumed from JSON
 */
export async function gotoAfterChainRegistration(): Promise<void> {
  console.log("✅ Continuing deployment from saved state...");

  try {
    // Run the verification step that gateway router was registered
    await verifyExecution();

    // Retrieve contract addresses
    retrieveMultisigAddresses();

    // Register and authorize MultisigProver
    await authorizeMultisigProver();
    
    // Save updated deployment config
    saveDeploymentConfig();
    
    console.log("🔍 Wait for multisig proposals to be approved...");
  } catch (error) {
    displayMessage(MessageType.ERROR, `Chain registration resume failed: ${error}`);
    throw error;
  }
}

/**
 * Function to handle the state after multisig proposals have been approved
 */
export async function gotoAfterMultisigProposals(): Promise<void> {
  try {
    await verifyMultisig();

    await createRewardPools();
    await addFundsToRewardPools();

    await createGenesisVerifierSet();

    await deployGatewayContract();

    console.log("🎉 Deployment complete!");
  } catch (error) {
    displayMessage(MessageType.ERROR, `Post-multisig proposals execution failed: ${error}`);
    throw error;
  }
}

/**
 * Function to print environment variables as JSON and exit
 */
export function printEnvJsonAndExit(): void {
  console.log("🎉 Chain registration complete! Need to Update the Verifiers!");
  
  // Save deployment config
  saveDeploymentConfig();
}