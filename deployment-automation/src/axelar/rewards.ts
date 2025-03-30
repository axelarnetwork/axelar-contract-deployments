/**
 * Reward pool functions
 */

import { execSync } from 'child_process';
import { config } from '../config/environment';
import { execAsync } from '../utils/exec';
import { isCustomDevnet } from '../config/network';
import { GAS_PRICE_COEFFICIENT } from '../../constants';

/**
 * Function to create reward pools
 */
export async function createRewardPools(): Promise<void> {
  console.log("⚡ Creating reward pools");
  
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
    
    const jsonCreatePoolVerifier = JSON.stringify({
      create_pool: {
        pool_id: {
          chain_name: config.CHAIN_NAME,
          contract: config.VOTING_VERIFIER_ADDRESS
        },
        params: JSON.parse(params)
      }
    });

    try {
      await execAsync(`axelard tx wasm execute "${config.REWARDS_ADDRESS}" '${jsonCreatePoolMultisig}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices ${GAS_PRICE_COEFFICIENT}${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);

      await execAsync(`axelard tx wasm execute "${config.REWARDS_ADDRESS}" '${jsonCreatePoolVerifier}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices ${GAS_PRICE_COEFFICIENT}${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
    } catch (error) {
      console.error(`Error creating reward pools: ${error}`);
      throw error;
    }
  } else {
    // Logic for submitting proposals through the NodeJS script
    if (config.NAMESPACE === "devnet-amplifier") {
      try {
        await execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Rewards \
          -t "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
          -d "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
          --runAs ${config.RUN_AS_ACCOUNT} \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg "{ \\"create_pool\\": { \\"params\\": { \\"epoch_duration\\": \\"${config.EPOCH_DURATION}\\", \\"participation_threshold\\": [\\"7\\", \\"10\\"], \\"rewards_per_epoch\\": \\"100\\" }, \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.VOTING_VERIFIER_ADDRESS}\\" } } }"`);
      } catch (error) {
        console.error(`Error creating reward pools via proposal (devnet-amplifier): ${error}`);
        throw error;
      }
    } else {
      try {
        await execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Rewards \
          -t "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
          -d "Create pool for ${config.CHAIN_NAME} in ${config.CHAIN_NAME} voting verifier" \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg "{ \\"create_pool\\": { \\"params\\": { \\"epoch_duration\\": \\"${config.EPOCH_DURATION}\\", \\"participation_threshold\\": [\\"7\\", \\"10\\"], \\"rewards_per_epoch\\": \\"100\\" }, \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.VOTING_VERIFIER_ADDRESS}\\" } } }"`);
      } catch (error) {
        console.error(`Error creating reward pools via proposal: ${error}`);
        throw error;
      }
    }
  }
}

/**
 * Function to add funds to reward pools
 */
export async function addFundsToRewardPools(): Promise<void> {
  if (!isCustomDevnet()) {
    console.log("⚡ Adding funds to reward pools...");
    
    try {
      const rewards = execSync(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq .axelar.contracts.Rewards.address | tr -d '"'`, { stdio: 'pipe' }).toString().trim();
      
      await execAsync(`axelard tx wasm execute ${rewards} "{ \\"add_rewards\\": { \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.MULTISIG_ADDRESS}\\" } } }" --amount ${config.REWARD_AMOUNT} --from ${config.WALLET_ADDRESS}`);
      
      await execAsync(`axelard tx wasm execute ${rewards} "{ \\"add_rewards\\": { \\"pool_id\\": { \\"chain_name\\": \\"${config.CHAIN_NAME}\\", \\"contract\\": \\"${config.VOTING_VERIFIER_ADDRESS}\\" } } }" --amount ${config.REWARD_AMOUNT} --from ${config.WALLET_ADDRESS}`);
    } catch (error) {
      console.error(`Error adding funds to reward pools: ${error}`);
      throw error;
    }
  }
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