/**
 * Multisig related functions
 */

import * as fs from 'fs';
import { execSync } from 'child_process';
import { config } from '../config/environment';
import { execAsync } from '../utils/exec';
import { isCustomDevnet } from '../config/network';
import { GAS_PRICE_COEFFICIENT } from '../../constants';

/**
 * Function to update the namespace JSON file with MultisigProver contract
 */
export function updateMultisigProver(): void {
  const namespaceJsonPath = `../axelar-chains-config/info/${config.NAMESPACE}.json`;

  // Check if the namespace JSON file exists
  if (!fs.existsSync(namespaceJsonPath)) {
    console.log(`❌ Namespace JSON file not found: ${namespaceJsonPath}`);
    throw new Error(`Namespace JSON file not found: ${namespaceJsonPath}`);
  }

  // Read the existing JSON file
  const existingJson = JSON.parse(fs.readFileSync(namespaceJsonPath, 'utf8'));

  // Check if "axelar.contracts.MultisigProver" exists in the JSON
  if (!existingJson.axelar?.contracts?.MultisigProver) {
    console.log(`❌ No 'MultisigProver' dictionary found in ${namespaceJsonPath}`);
    throw new Error(`No 'MultisigProver' dictionary found in ${namespaceJsonPath}`);
  }

  // Check if CHAIN_NAME already exists in "MultisigProver"
  if (existingJson.axelar.contracts.MultisigProver[config.CHAIN_NAME!]) {
    console.log(`❌ Chain '${config.CHAIN_NAME}' already exists under 'MultisigProver' in ${namespaceJsonPath}! Aborting to prevent overwriting.`);
    throw new Error(`Chain '${config.CHAIN_NAME}' already exists under 'MultisigProver'`);
  }

  // Create the new chain entry with updated environment variables
  const newMultisigProverEntry = {
    governanceAddress: config.GOVERNANCE_ADDRESS,
    adminAddress: config.ADMIN_ADDRESS,
    destinationChainID: config.CHAIN_ID,
    signingThreshold: JSON.parse(config.SIGNING_THRESHOLD!),
    serviceName: config.SERVICE_NAME,
    verifierSetDiffThreshold: 0,
    encoder: "abi",
    keyType: "ecdsa"
  };

  // Insert the new chain entry into "MultisigProver"
  existingJson.axelar.contracts.MultisigProver[config.CHAIN_NAME!] = newMultisigProverEntry;

  // Write back the updated JSON
  fs.writeFileSync(namespaceJsonPath, JSON.stringify(existingJson, null, 2));
  console.log(`✅ Successfully added '${config.CHAIN_NAME}' under 'MultisigProver' in ${namespaceJsonPath}`);

  // Confirm the new entry was added
  console.log("🔍 Verifying the new MultisigProver entry...");
  console.log(JSON.stringify(existingJson.axelar.contracts.MultisigProver, null, 2));
}

/**
 * Function to verify multisig
 */
export async function verifyMultisig(): Promise<void> {
  console.log("⚡ Verifying the transaction execution for MultisigProver...");

  const jsonQuery = JSON.stringify({
    is_caller_authorized: {
      contract_address: config.MULTISIG_PROVER_ADDRESS,
      chain_name: config.CHAIN_NAME
    }
  });

  try {
    const { stdout } = await execAsync(`axelard q wasm contract-state smart "${config.MULTISIG_ADDRESS}" '${jsonQuery}' --node "${config.AXELAR_RPC_URL}"`);
    
    // Print raw output for debugging
    console.log("🔍 Verification Output:");
    console.log(stdout);

    // Check if the output contains "data: true" as plain text
    if (stdout.includes("data: true")) {
      console.log("✅ Verification successful! MultisigProver is authorized.");
    } else {
      console.log("❌ Verification failed! Expected 'data: true' but got:");
      console.log(stdout);
      throw new Error("MultisigProver verification failed");
    }
  } catch (error) {
    console.error(`Error during multisig verification: ${error}`);
    throw error;
  }
}

/**
 * Function to create genesis verifier set
 */
export async function createGenesisVerifierSet(): Promise<void> {
  try {
    await execAsync(`axelard tx wasm execute ${config.MULTISIG_PROVER_ADDRESS} '"update_verifier_set"' \
      --from ${config.PROVER_ADMIN} \
      --gas auto \
      --gas-adjustment 2 \
      --node "${config.AXELAR_RPC_URL}" \
      --gas-prices ${GAS_PRICE_COEFFICIENT}${config.TOKEN_DENOM} \
      --keyring-backend test \
      --chain-id "${config.NAMESPACE}"`);
    
    console.log("🔍 Querying multisig prover for active verifier set...");
    
    const { stdout } = await execAsync(`axelard q wasm contract-state smart ${config.MULTISIG_PROVER_ADDRESS} '"current_verifier_set"' \
      --node "${config.AXELAR_RPC_URL}" \
      --chain-id "${config.NAMESPACE}"`);
    
    console.log(stdout);
  } catch (error) {
    console.error(`Error creating genesis verifier set: ${error}`);
    throw error;
  }
}

/**
 * Function to retrieve contract addresses
 */
export function retrieveMultisigAddresses(): void {
  try {
    // Retrieve the Multisig Contract Address
    const multisigAddress = execSync(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '.axelar.contracts.Multisig.address'`, { stdio: 'pipe' }).toString().trim();
    config.MULTISIG_ADDRESS = multisigAddress;
    console.log(`✅ Retrieved MULTISIG_ADDRESS: ${multisigAddress}`);

    // Retrieve the Multisig Prover Contract Address
    const query = `.axelar.contracts.MultisigProver.${config.CHAIN_NAME}.address`;
    const multisigProverAddress = execSync(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '${query}'`, { stdio: 'pipe' }).toString().trim();
    config.MULTISIG_PROVER_ADDRESS = multisigProverAddress;
    console.log(`✅ Retrieved MULTISIG_PROVER_ADDRESS: ${multisigProverAddress}`);

    // Retrieve coordinator address
    const coordinatorAddress = execSync(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '.axelar.contracts.Coordinator.address'`, { stdio: 'pipe' }).toString().trim();
    config.COORDINATOR_ADDRESS = coordinatorAddress;
    console.log(`✅ Retrieved COORDINATOR_ADDRESS: ${coordinatorAddress}`);
  } catch (error) {
    console.error(`Error retrieving multisig addresses: ${error}`);
    throw error;
  }
}

/**
 * Function to authorize MultisigProver as a caller
 */
export async function authorizeMultisigProver(): Promise<void> {
  // Construct JSON Payload for the Execute Call
  const jsonCmdMultisig = JSON.stringify({
    authorize_callers: {
      contracts: {
        [config.MULTISIG_PROVER_ADDRESS!]: config.CHAIN_NAME
      }
    }
  });
  console.log(`📜 JSON Command for Multisig: ${jsonCmdMultisig}`);

  // Prepare JSON for registering the prover contract
  const jsonCmdMultisigProver = JSON.stringify({
    register_prover_contract: {
      chain_name: config.CHAIN_NAME,
      new_prover_addr: config.MULTISIG_PROVER_ADDRESS
    }
  });
  console.log(`📜 JSON Command for Coordinator: ${jsonCmdMultisigProver}`);

  if (isCustomDevnet()) {
    console.log("Register prover contract");

    try {
      await execAsync(`axelard tx wasm execute "${config.COORDINATOR_ADDRESS}" '${jsonCmdMultisigProver}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices ${GAS_PRICE_COEFFICIENT}${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);

      // Execute the Transaction for Multisig Contract
      console.log("⚡ Executing authorize_callers for Multisig Contract...");

      await execAsync(`axelard tx wasm execute "${config.MULTISIG_ADDRESS}" '${jsonCmdMultisig}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices ${GAS_PRICE_COEFFICIENT}${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
    } catch (error) {
      console.error(`Error registering prover contract: ${error}`);
      throw error;
    }
  } else {
    // Actual networks require proposal for chain integration
    try {
      if (config.NAMESPACE === "devnet-amplifier") {
        await execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Coordinator \
          -t "Register Multisig Prover for ${config.CHAIN_NAME}" \
          -d "Register Multisig Prover address for ${config.CHAIN_NAME} at Coordinator contract" \
          --runAs ${config.RUN_AS_ACCOUNT} \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg '${jsonCmdMultisigProver}'`);
        
        await execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Multisig \
          -t "Authorize Multisig Prover for ${config.CHAIN_NAME}" \
          -d "Authorize Multisig Prover address for ${config.CHAIN_NAME} at Multisig contract" \
          --runAs ${config.RUN_AS_ACCOUNT} \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg '${jsonCmdMultisig}'`);
      } else {
        await execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Coordinator \
          -t "Register Multisig Prover for ${config.CHAIN_NAME}" \
          -d "Register Multisig Prover address for ${config.CHAIN_NAME} at Coordinator contract" \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg '${jsonCmdMultisigProver}'`);

        await execAsync(`node ../cosmwasm/submit-proposal.js execute \
          -c Multisig \
          -t "Authorize Multisig Prover for ${config.CHAIN_NAME}" \
          -d "Authorize Multisig Prover address for ${config.CHAIN_NAME} at Multisig contract" \
          --deposit ${config.DEPOSIT_VALUE} \
          --msg '${jsonCmdMultisig}'`);
      }
    } catch (error) {
      console.error(`Error submitting proposals: ${error}`);
      throw error;
    }
  }
}