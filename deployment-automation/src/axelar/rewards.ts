/**
 * Reward pool functions
 */

import { execSync } from 'child_process';
import { config } from '../config/environment';
import { execAsync } from '../utils/exec';
import { isCustomDevnet } from '../config/network';
import { GAS_PRICE_COEFFICIENT } from '../../constants';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Helper function to extract proposal ID from command output
 */
function extractProposalId(output: string): number | undefined {
  const proposalIdMatch = output.match(/Proposal submitted: (\d+)/);
  return proposalIdMatch ? parseInt(proposalIdMatch[1], 10) : undefined;
}

/**
 * Creates a reward pool for Voting Verifier via proposal
 * @returns The proposal ID if submitted successfully
 */
export async function createVotingVerifierRewardPool(): Promise<number | void> {
  console.log(`⚡ Creating reward pool for Voting Verifier (${config.CHAIN_NAME})`);
  
  if (isCustomDevnet()) {
    const params = JSON.stringify({
      epoch_duration: "10",
      rewards_per_epoch: "100",
      participation_threshold: ["9", "10"]
    });
    
    const jsonCreatePoolVerifier = JSON.stringify({
      create_pool: {
        pool_id: {
          chain_name: config.CHAIN_NAME,
          contract: config.VOTING_VERIFIER_ADDRESS
        },
        params: JSON.parse(params)
      }
    });

    // Create verifier pool
    try {
      await execAsync(`axelard tx wasm execute "${config.REWARDS_ADDRESS}" '${jsonCreatePoolVerifier}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices ${GAS_PRICE_COEFFICIENT}${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
      
      console.log("✅ Created voting verifier reward pool");
    } catch (error) {
      // Check if the error is because the pool already exists
      const errorStr = String(error);
      if (errorStr.includes("rewards pool already exists")) {
        console.log("⚠️ Voting verifier rewards pool already exists. Continuing...");
      } else {
        console.error(`Error creating voting verifier reward pool: ${error}`);
        throw error;
      }
    }
  } else {
    // Logic for submitting proposals through the NodeJS script
    const command = config.NAMESPACE === "devnet-amplifier"
      ? `node ../cosmwasm/submit-proposal.js execute \
        -c Rewards \
        -t "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
        -d "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
        -e "${config.NAMESPACE}" -y \
        --runAs ${config.RUN_AS_ACCOUNT} \
        --deposit ${config.DEPOSIT_VALUE} \
        --msg "{ \\"create_pool\\": { \\"params\\": { \\"epoch_duration\\": \\"${config.EPOCH_DURATION}\\", \\"participation_threshold\\": [\\"7\\", \\"10\\"], \\"rewards_per_epoch\\": \\"100\\" }, \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.VOTING_VERIFIER_ADDRESS}\\" } } }"`
      : `node ../cosmwasm/submit-proposal.js execute \
        -c Rewards \
        -t "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
        -d "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
         -e "${config.NAMESPACE}" -y \
        --deposit ${config.DEPOSIT_VALUE} \
        --msg "{ \\"create_pool\\": { \\"params\\": { \\"epoch_duration\\": \\"${config.EPOCH_DURATION}\\", \\"participation_threshold\\": [\\"7\\", \\"10\\"], \\"rewards_per_epoch\\": \\"100\\" }, \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.VOTING_VERIFIER_ADDRESS}\\" } } }"`;
    
    try {
      // Submit proposal and capture output
      const { stdout, stderr } = await execAsync(command, { maxBuffer: 1024 * 1024 * 10 });
      
      // Log the complete command output
      console.log(`\n==== VOTING VERIFIER REWARD POOL PROPOSAL OUTPUT START ====`);
      console.log(stdout);
      if (stderr) {
        console.error(`==== STDERR OUTPUT ====`);
        console.error(stderr);
      }
      console.log(`==== VOTING VERIFIER REWARD POOL PROPOSAL OUTPUT END ====\n`);
      
      // Extract the proposal ID
      const proposalId = extractProposalId(stdout);
      if (proposalId) {
        console.log(`✅ Voting verifier reward pool proposal #${proposalId} submitted for ${config.CHAIN_NAME}`);
        
        // Save proposal ID to config
        config.CREATE_VOTING_VERIFIER_REWARD_POOL_PROPOSAL_ID = proposalId.toString();
        
        // Save proposal output to file for record keeping
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const logFilePath = `./logs/voting-verifier-reward-pool-${config.CHAIN_NAME}-${timestamp}.log`;
        await fs.promises.mkdir('./logs', { recursive: true });
        await fs.promises.writeFile(logFilePath, 
          `Command: ${command}\n\n` +
          `STDOUT:\n${stdout}\n\n` +
          (stderr ? `STDERR:\n${stderr}\n\n` : '') +
          `Timestamp: ${new Date().toISOString()}\n` +
          `Proposal ID: ${proposalId}`
        );
        console.log(`📄 Voting verifier reward pool proposal output saved to ${logFilePath}`);
        
        return proposalId;
      }
    } catch (error) {
      // Check if the error is because the pool already exists
      const errorStr = String(error);
      if (errorStr.includes("rewards pool already exists")) {
        console.log("⚠️ Voting verifier reward pool already exists. Continuing...");
      } else {
        console.error(`Error creating voting verifier reward pool via proposal: ${error}`);
        throw error;
      }
    }
  }
}

/**
 * Creates a reward pool for Multisig via proposal
 * @returns The proposal ID if submitted successfully
 */
export async function createMultisigRewardPool(): Promise<number | void> {
  console.log(`⚡ Creating reward pool for Multisig (${config.CHAIN_NAME})`);
  
  if (isCustomDevnet()) {
    const params = JSON.stringify({
      epoch_duration: "10",
      rewards_per_epoch: "100",
      participation_threshold: ["9", "10"]
    });
    
    const jsonCreatePoolMultisig = JSON.stringify({
      create_pool: {
        pool_id: {
          chain_name: config.CHAIN_NAME,
          contract: config.MULTISIG_ADDRESS
        },
        params: JSON.parse(params)
      }
    });

    // Create multisig pool
    try {
      await execAsync(`axelard tx wasm execute "${config.REWARDS_ADDRESS}" '${jsonCreatePoolMultisig}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices ${GAS_PRICE_COEFFICIENT}${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
      
      console.log("✅ Created multisig reward pool");
    } catch (error) {
      // Check if the error is because the pool already exists
      const errorStr = String(error);
      if (errorStr.includes("rewards pool already exists")) {
        console.log("⚠️ Multisig rewards pool already exists. Continuing...");
      } else {
        console.error(`Error creating multisig reward pool: ${error}`);
        throw error;
      }
    }
  } else {
    // Logic for submitting proposals through the NodeJS script
    const command = config.NAMESPACE === "devnet-amplifier"
      ? `node ../cosmwasm/submit-proposal.js execute \
        -c Rewards \
        -t "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} multisig" \
        -d "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} multisig" \
        --runAs ${config.RUN_AS_ACCOUNT} \
        --deposit ${config.DEPOSIT_VALUE} \
         -e "${config.NAMESPACE}" -y \
        --msg "{ \\"create_pool\\": { \\"params\\": { \\"epoch_duration\\": \\"${config.EPOCH_DURATION}\\", \\"participation_threshold\\": [\\"7\\", \\"10\\"], \\"rewards_per_epoch\\": \\"100\\" }, \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.MULTISIG_ADDRESS}\\" } } }"`
      : `node ../cosmwasm/submit-proposal.js execute \
        -c Rewards \
        -t "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} multisig" \
        -d "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} multisig" \
        --deposit ${config.DEPOSIT_VALUE} \
         -e "${config.NAMESPACE}" -y \
        --msg "{ \\"create_pool\\": { \\"params\\": { \\"epoch_duration\\": \\"${config.EPOCH_DURATION}\\", \\"participation_threshold\\": [\\"7\\", \\"10\\"], \\"rewards_per_epoch\\": \\"100\\" }, \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.MULTISIG_ADDRESS}\\" } } }"`;
    
    try {
      // Submit proposal and capture output
      const { stdout, stderr } = await execAsync(command, { maxBuffer: 1024 * 1024 * 10 });
      
      // Log the complete command output
      console.log(`\n==== MULTISIG REWARD POOL PROPOSAL OUTPUT START ====`);
      console.log(stdout);
      if (stderr) {
        console.error(`==== STDERR OUTPUT ====`);
        console.error(stderr);
      }
      console.log(`==== MULTISIG REWARD POOL PROPOSAL OUTPUT END ====\n`);
      
      // Extract the proposal ID
      const proposalId = extractProposalId(stdout);
      if (proposalId) {
        console.log(`✅ Multisig reward pool proposal #${proposalId} submitted for ${config.CHAIN_NAME}`);
        
        // Save proposal ID to config
        config.CREATE_MULTISIG_REWARD_POOL_PROPOSAL_ID = proposalId.toString();
        
        // Save proposal output to file for record keeping
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const logFilePath = `./logs/multisig-reward-pool-${config.CHAIN_NAME}-${timestamp}.log`;
        await fs.promises.mkdir('./logs', { recursive: true });
        await fs.promises.writeFile(logFilePath, 
          `Command: ${command}\n\n` +
          `STDOUT:\n${stdout}\n\n` +
          (stderr ? `STDERR:\n${stderr}\n\n` : '') +
          `Timestamp: ${new Date().toISOString()}\n` +
          `Proposal ID: ${proposalId}`
        );
        console.log(`📄 Multisig reward pool proposal output saved to ${logFilePath}`);
        
        return proposalId;
      }
    } catch (error) {
      // Check if the error is because the pool already exists
      const errorStr = String(error);
      if (errorStr.includes("rewards pool already exists")) {
        console.log("⚠️ Multisig reward pool already exists. Continuing...");
      } else {
        console.error(`Error creating multisig reward pool via proposal: ${error}`);
        throw error;
      }
    }
  }
}

/**
 * Legacy function that creates both reward pools (for backward compatibility)
 */
export async function createRewardPools(): Promise<void> {
  console.log("⚡ Creating reward pools");
  
  // Call the individual functions to create each pool
  await createMultisigRewardPool();
  await createVotingVerifierRewardPool();
}

/**
 * Adds funds to the Voting Verifier reward pool
 */
export async function addFundsToVotingVerifierRewardPool(): Promise<void> {
  if (isCustomDevnet()) {
    console.log("⚠️ Skipping reward pool funding in custom devnet environment");
    return;
  }
  
  console.log("⚡ Adding funds to Voting Verifier reward pool...");
  
  try {
    // Ensure we have the rewards address
    if (!config.REWARDS_ADDRESS) {
      retrieveRewardsAddress();
    }
    
    // Ensure TOKEN_DENOM is defined
    if (!config.TOKEN_DENOM) {
      console.error("❌ TOKEN_DENOM is not defined in the configuration");
      throw new Error("TOKEN_DENOM is not defined");
    }
    

    
    // Proceed with adding funds
    try {
      await execAsync(`axelard tx wasm execute ${config.REWARDS_ADDRESS} \
        "{ \\"add_rewards\\": { \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.VOTING_VERIFIER_ADDRESS}\\" } } }" \
        --amount ${config.REWARD_AMOUNT}${config.TOKEN_DENOM} \
        --from ${config.WALLET_ADDRESS} \
        --node "${config.AXELAR_RPC_URL}" \
        --gas auto \
        --gas-adjustment 2 \
        --gas-prices 0.00005${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);

      console.log("✅ Added funds to voting verifier reward pool");
    } catch (error) {
      // Check if it's a non-critical error
      const errorStr = String(error);
      if (errorStr.includes("rewards already added") || errorStr.includes("insufficient funds")) {
        console.log("⚠️ Error adding funds:", errorStr);
        console.log("⚠️ Could not add funds to voting verifier reward pool. Continuing...");
      } else {
        throw error;
      }
    }
  } catch (error) {
    console.error(`Error adding funds to voting verifier reward pool: ${error}`);
    console.log("⚠️ Continuing despite voting verifier reward pool funding issues...");
  }
}

/**
 * Adds funds to the Multisig reward pool
 */
export async function addFundsToMultisigRewardPool(): Promise<void> {
  if (isCustomDevnet()) {
    console.log("⚠️ Skipping reward pool funding in custom devnet environment");
    return;
  }
  
  console.log("⚡ Adding funds to Multisig reward pool...");
  
  try {
    // Ensure we have the rewards address
    if (!config.REWARDS_ADDRESS) {
      retrieveRewardsAddress();
    }
    
    // Ensure TOKEN_DENOM is defined
    if (!config.TOKEN_DENOM) {
      console.error("❌ TOKEN_DENOM is not defined in the configuration");
      throw new Error("TOKEN_DENOM is not defined");
    }

    
    // Proceed with adding funds
    try {
      await execAsync(`axelard tx wasm execute ${config.REWARDS_ADDRESS} \
        "{ \\"add_rewards\\": { \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.MULTISIG_ADDRESS}\\" } } }" \
        --amount ${config.REWARD_AMOUNT}${config.TOKEN_DENOM} \
        --from ${config.WALLET_ADDRESS} \
        --node "${config.AXELAR_RPC_URL}" \
        --gas auto \
        --gas-adjustment 2 \
        --gas-prices 0.00005${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
      
      console.log("✅ Added funds to multisig reward pool");
    } catch (error) {
      // Check if it's a non-critical error
      const errorStr = String(error);
      if (errorStr.includes("rewards already added") || errorStr.includes("insufficient funds")) {
        console.log("⚠️ Error adding funds:", errorStr);
        console.log("⚠️ Could not add funds to multisig reward pool. Continuing...");
      } else {
        throw error;
      }
    }
  } catch (error) {
    console.error(`Error adding funds to multisig reward pool: ${error}`);
    console.log("⚠️ Continuing despite multisig reward pool funding issues...");
  }
}

/**
 * Legacy function that adds funds to both reward pools (for backward compatibility)
 */
export async function addFundsToRewardPools(): Promise<void> {
  console.log("⚡ Adding funds to reward pools...");
  
  // Call the individual functions to fund each pool
  await addFundsToMultisigRewardPool();
  await addFundsToVotingVerifierRewardPool();
}

/**
 * Retrieve rewards address
 */
export function retrieveRewardsAddress(): void {
  try {
    const rewards = execSync(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '.axelar.contracts.Rewards.address'`, { stdio: 'pipe' }).toString().trim();
    config.REWARDS_ADDRESS = rewards;
    console.log(`✅ Retrieved REWARDS_ADDRESS: ${rewards}`);
  } catch (error) {
    console.error(`Error retrieving rewards address: ${error}`);
    throw error;
  }
}

/**
 * Updates the saveDeploymentConfig function to include reward pool proposal IDs
 * This function should be imported and called in the saveDeploymentConfig function
 */
export function addRewardPoolProposalIdsToConfigKeys(configKeys: string[]): string[] {
  return [
    ...configKeys,
    'CREATE_VOTING_VERIFIER_REWARD_POOL_PROPOSAL_ID',
    'CREATE_MULTISIG_REWARD_POOL_PROPOSAL_ID'
  ];
}