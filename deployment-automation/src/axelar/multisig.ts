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
 * Function to update MultisigProver inside axelar.contracts in JSON config
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

  // Handle signing threshold parsing properly
  let signingThreshold = ["6", "10"]; // Default value
  
  if (config.SIGNING_THRESHOLD) {
    try {
      // If it's already an array, use it directly
      if (Array.isArray(config.SIGNING_THRESHOLD)) {
        signingThreshold = config.SIGNING_THRESHOLD;
      } 
      // If it's a string representation of an array
      else if (typeof config.SIGNING_THRESHOLD === 'string') {
        // Check if the string starts with [ and ends with ]
        if (config.SIGNING_THRESHOLD.trim().startsWith('[') && config.SIGNING_THRESHOLD.trim().endsWith(']')) {
          signingThreshold = JSON.parse(config.SIGNING_THRESHOLD);
        } 
        // If it's a comma-separated string like "6,10"
        else if (config.SIGNING_THRESHOLD.includes(',')) {
          signingThreshold = config.SIGNING_THRESHOLD.split(',').map(item => item.trim());
        }
        // If it's just a single value like "6"
        else {
          console.log(`⚠️ SIGNING_THRESHOLD is a single value: ${config.SIGNING_THRESHOLD}. Using ["${config.SIGNING_THRESHOLD}", "10"]`);
          signingThreshold = [config.SIGNING_THRESHOLD, "10"];
        }
      }
    } catch (error) {
      console.error(`Error parsing SIGNING_THRESHOLD (${config.SIGNING_THRESHOLD}): ${error}`);
      console.log(`⚠️ Using default SIGNING_THRESHOLD: ${JSON.stringify(signingThreshold)}`);
    }
  }
  
  console.log(`✅ Using SIGNING_THRESHOLD: ${JSON.stringify(signingThreshold)}`);

  // Create the new chain entry with updated environment variables
  const newMultisigProverEntry = {
    governanceAddress: config.GOVERNANCE_ADDRESS,
    adminAddress: config.ADMIN_ADDRESS,
    destinationChainID: config.CHAIN_ID,
    signingThreshold: signingThreshold,
    serviceName: config.SERVICE_NAME || "validators",
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
  console.log(JSON.stringify(existingJson.axelar.contracts.MultisigProver[config.CHAIN_NAME!], null, 2));
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
        
      
      console.log("✅ Successfully updated verifier set");
      
    } catch (error) {
      // Check for specific error about verifier set not changing
      const errorStr = String(error);
      if (errorStr.includes("verifier set has not changed sufficiently since last update")) {
        console.log("⚠️ Verifier set has not changed sufficiently. This is normal if it was recently updated.");
        // We can continue as this is not a fatal error
      } else {
        console.error(`Error creating genesis verifier set: ${error}`);
        throw error;
      }
    }
  
    // Query the current verifier set regardless of whether update succeeded
    try {
      console.log("🔍 Querying multisig prover for active verifier set...");
      
      const { stdout } = await execAsync(`axelard q wasm contract-state smart ${config.MULTISIG_PROVER_ADDRESS} '"current_verifier_set"' \
        --node "${config.AXELAR_RPC_URL}" \
        --chain-id "${config.NAMESPACE}"`);
      
      console.log(stdout);
      
      // Validate the verifier set
      const signerAddressRegex = /address: axelar[a-z0-9]+/g;
      const matches = stdout.match(signerAddressRegex) || [];
      const numSigners = matches.length;
      
      // Extract signer weights if available
      const weightRegex = /weight: "(\d+)"/g;
      const weightMatches = [...stdout.matchAll(weightRegex)];
      let totalWeight = 0;
      
      if (weightMatches.length > 0) {
        totalWeight = weightMatches.reduce((sum, match) => sum + parseInt(match[1]), 0);
        console.log(`✅ Total signer weight: ${totalWeight}`);
      }
      
      if (numSigners === 0) {
        console.error("❌ No signers found in verifier set!");
        throw new Error("Verifier set validation failed: No signers found");
      } else if (numSigners < 2) {
        console.warn("⚠️ Only one signer found in verifier set. This may be insufficient for secure operation.");
      } else {
        console.log(`✅ Found ${numSigners} signers in verifier set.`);
      }
      
      // Try to extract threshold
      const thresholdMatch = stdout.match(/threshold: "(\d+)"/);
      if (thresholdMatch) {
        const threshold = parseInt(thresholdMatch[1]);
        console.log(`✅ Threshold set to ${threshold} of ${numSigners} signers.`);
        
        if (threshold > numSigners) {
          console.error(`❌ Threshold (${threshold}) is greater than the number of signers (${numSigners})!`);
          throw new Error("Invalid threshold configuration: Threshold greater than signer count");
        }
        
        if (weightMatches.length > 0 && threshold > totalWeight) {
          console.error(`❌ Threshold (${threshold}) is greater than the total signer weight (${totalWeight})!`);
          throw new Error("Invalid threshold configuration: Threshold greater than total weight");
        }
      } else {
        console.warn("⚠️ Could not extract threshold from verifier set output.");
      }
      
      // Check if there are sufficient signers to ever meet the threshold
      if (thresholdMatch) {
        const threshold = parseInt(thresholdMatch[1]);
        
        // In a simple case where each signer has weight 1
        if (numSigners < threshold) {
          console.error(`❌ Not enough signers (${numSigners}) to meet the threshold (${threshold})!`);
          throw new Error("Insufficient signers to meet threshold");
        }
        
        // If we have weight information, use it for a more accurate check
        if (weightMatches.length > 0) {
          // Sort signer weights in descending order to calculate best-case scenario
          const weights = weightMatches.map(match => parseInt(match[1])).sort((a, b) => b - a);
          
          // Calculate how many of the highest-weight signers are needed to meet threshold
          let weightSum = 0;
          let signersNeeded = 0;
          
          for (const weight of weights) {
            weightSum += weight;
            signersNeeded++;
            
            if (weightSum >= threshold) {
              break;
            }
          }
          
          console.log(`✅ Minimum signers needed to meet threshold: ${signersNeeded}`);
          
          // Check if it's impossible to meet the threshold even with all signers
          if (weightSum < threshold) {
            console.error(`❌ Even with all signers, the total weight (${totalWeight}) is less than the threshold (${threshold})!`);
            throw new Error("Total signer weight insufficient to meet threshold");
          }
        }
      }
      
    } catch (error: unknown) {
        // Properly type the error
        const queryError = error as Error;
        
        if (typeof queryError === 'object' && 
            queryError !== null && 
            'message' in queryError && 
            typeof queryError.message === 'string') {
          
          if (queryError.message.includes("Insufficient signers") || 
              queryError.message.includes("Invalid threshold") ||
              queryError.message.includes("No signers found")) {
            // Rethrow validation errors
            throw queryError;
          }
        }
        
        console.warn(`Warning: Could not query or validate current verifier set: ${error}`);
        // We don't throw here for generic query errors because querying the verifier set is informational
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

// Helper function to extract proposal ID from command output
function extractProposalId(output: string): number {
  const proposalIdMatch = output.match(/Proposal submitted: (\d+)/);
  const proposalId = proposalIdMatch ? parseInt(proposalIdMatch[1], 10) : null;
  
  if (proposalId === null) {
    throw new Error('Could not extract proposal ID from command output');
  }
  
  return proposalId;
}

/**
 * Registers the multisig prover with the Coordinator contract
 * @returns The proposal ID for the Coordinator registration if applicable
 */
export async function registerMultisigProverWithCoordinator(): Promise<number | void> {
  // Prepare JSON for registering the prover contract
  const jsonCmdMultisigProver = JSON.stringify({
    register_prover_contract: {
      chain_name: config.CHAIN_NAME,
      new_prover_addr: config.MULTISIG_PROVER_ADDRESS
    }
  });
  console.log(`📜 JSON Command for Coordinator: ${jsonCmdMultisigProver}`);

  if (isCustomDevnet()) {
    console.log("Register prover contract with Coordinator");

    try {
      await execAsync(`axelard tx wasm execute "${config.COORDINATOR_ADDRESS}" '${jsonCmdMultisigProver}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices ${GAS_PRICE_COEFFICIENT}${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}" \
        -e "${config.NAMESPACE}"`);
        
      console.log(`✅ Multisig prover registered with Coordinator for ${config.CHAIN_NAME}`);
    } catch (error) {
      console.error(`Error registering prover contract with Coordinator: ${error}`);
      throw error;
    }
  } else {
    // Actual networks require proposal for chain integration
    try {
      const command = config.NAMESPACE === "devnet-amplifier"
        ? `node ../cosmwasm/submit-proposal.js execute \
            -c Coordinator \
            -t "Register Multisig Prover for ${config.CHAIN_NAME}" \
            -d "Register Multisig Prover address for ${config.CHAIN_NAME} at Coordinator contract" \
            --runAs ${config.RUN_AS_ACCOUNT} \
            --deposit ${config.DEPOSIT_VALUE} \
            -e "${config.NAMESPACE}" \
            --msg '${jsonCmdMultisigProver}' -y`
        : `node ../cosmwasm/submit-proposal.js execute \
            -c Coordinator \
            -t "Register Multisig Prover for ${config.CHAIN_NAME}" \
            -d "Register Multisig Prover address for ${config.CHAIN_NAME} at Coordinator contract" \
            --deposit ${config.DEPOSIT_VALUE} \
            -e "${config.NAMESPACE}" \
            --msg '${jsonCmdMultisigProver}' -y`;
      
      // Submit proposal for Coordinator contract and capture output
      const { stdout, stderr } = await execAsync(command, { maxBuffer: 1024 * 1024 * 10 });
      
      // Log the complete command output
      console.log(`\n==== COORDINATOR PROPOSAL OUTPUT START ====`);
      console.log(stdout);
      if (stderr) {
        console.error(`==== STDERR OUTPUT ====`);
        console.error(stderr);
      }
      console.log(`==== COORDINATOR PROPOSAL OUTPUT END ====\n`);
      
      // Extract the proposal ID
      const coordinatorProposalId = extractProposalId(stdout);
      console.log(`✅ Coordinator proposal #${coordinatorProposalId} submitted for ${config.CHAIN_NAME}`);
      
      // Save proposal ID to config
      config.REGISTER_MULTISIG_PROVER_COORDINATOR_PROPOSAL_ID = coordinatorProposalId.toString();
      
      // Save proposal output to file for record keeping
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const logFilePath = `./logs/coordinator-proposal-${config.CHAIN_NAME}-${timestamp}.log`;
      await fs.promises.mkdir('./logs', { recursive: true });
      await fs.promises.writeFile(logFilePath, 
        `Command: ${command}\n\n` +
        `STDOUT:\n${stdout}\n\n` +
        (stderr ? `STDERR:\n${stderr}\n\n` : '') +
        `Timestamp: ${new Date().toISOString()}\n` +
        `Proposal ID: ${coordinatorProposalId}`
      );
      console.log(`📄 Coordinator proposal output saved to ${logFilePath}`);
      
      return coordinatorProposalId;
    } catch (error) {
      console.error(`Error submitting coordinator proposal: ${error}`);
      throw error;
    }
  }
}

/**
 * Authorizes the multisig prover with the Multisig contract
 * @returns The proposal ID for the Multisig authorization if applicable
 */
export async function authorizeMultisigProver(): Promise<number | void> {
  // Construct JSON Payload for the Execute Call
  const jsonCmdMultisig = JSON.stringify({
    authorize_callers: {
      contracts: {
        [config.MULTISIG_PROVER_ADDRESS!]: config.CHAIN_NAME
      }
    }
  });
  console.log(`📜 JSON Command for Multisig: ${jsonCmdMultisig}`);

  if (isCustomDevnet()) {
    // Execute the Transaction for Multisig Contract
    console.log("⚡ Executing authorize_callers for Multisig Contract...");

    try {
      await execAsync(`axelard tx wasm execute "${config.MULTISIG_ADDRESS}" '${jsonCmdMultisig}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices ${GAS_PRICE_COEFFICIENT}${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
        
      console.log(`✅ Multisig prover authorized for ${config.CHAIN_NAME}`);
    } catch (error) {
      console.error(`Error authorizing multisig prover: ${error}`);
      throw error;
    }
  } else {
    // Actual networks require proposal for chain integration
    try {
      const command = config.NAMESPACE === "devnet-amplifier"
        ? `node ../cosmwasm/submit-proposal.js execute \
            -c Multisig \
            -t "Authorize Multisig Prover for ${config.CHAIN_NAME}" \
            -d "Authorize Multisig Prover address for ${config.CHAIN_NAME} at Multisig contract" \
            --runAs ${config.RUN_AS_ACCOUNT} \
            --deposit ${config.DEPOSIT_VALUE} \
            -e "${config.NAMESPACE}" \
            --msg '${jsonCmdMultisig}' -y`
        : `node ../cosmwasm/submit-proposal.js execute \
            -c Multisig \
            -t "Authorize Multisig Prover for ${config.CHAIN_NAME}" \
            -d "Authorize Multisig Prover address for ${config.CHAIN_NAME} at Multisig contract" \
            --deposit ${config.DEPOSIT_VALUE} \
            -e "${config.NAMESPACE}" \
            --msg '${jsonCmdMultisig}' -y`;
      
      // Submit proposal for Multisig contract and capture output
      const { stdout, stderr } = await execAsync(command, { maxBuffer: 1024 * 1024 * 10 });
      
      // Log the complete command output
      console.log(`\n==== MULTISIG PROPOSAL OUTPUT START ====`);
      console.log(stdout);
      if (stderr) {
        console.error(`==== STDERR OUTPUT ====`);
        console.error(stderr);
      }
      console.log(`==== MULTISIG PROPOSAL OUTPUT END ====\n`);
      
      // Extract the proposal ID
      const multisigProposalId = extractProposalId(stdout);
      console.log(`✅ Multisig proposal #${multisigProposalId} submitted for ${config.CHAIN_NAME}`);
      
      // Save proposal ID to config
      config.AUTHORIZE_MULTISIG_PROVER_PROPOSAL_ID = multisigProposalId.toString();
      
      // Save proposal output to file for record keeping
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      const logFilePath = `./logs/multisig-proposal-${config.CHAIN_NAME}-${timestamp}.log`;
      await fs.promises.mkdir('./logs', { recursive: true });
      await fs.promises.writeFile(logFilePath, 
        `Command: ${command}\n\n` +
        `STDOUT:\n${stdout}\n\n` +
        (stderr ? `STDERR:\n${stderr}\n\n` : '') +
        `Timestamp: ${new Date().toISOString()}\n` +
        `Proposal ID: ${multisigProposalId}`
      );
      console.log(`📄 Multisig proposal output saved to ${logFilePath}`);
      
      return multisigProposalId;
    } catch (error) {
      console.error(`Error submitting multisig proposal: ${error}`);
      throw error;
    }
  }
}