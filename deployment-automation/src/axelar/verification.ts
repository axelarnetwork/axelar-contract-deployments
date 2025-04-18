/**
 * Verification functions
 */

import * as fs from 'fs';
import { config } from '../config/environment';
import { execAsync } from '../utils/exec';

/**
 * Function to update VotingVerifier inside axelar.contracts in JSON config
 */
export function updateVotingVerifierConfig(): void {
  console.log("⚡ Updating VotingVerifier configuration...");

  const networkJsonPath = `../axelar-chains-config/info/${config.NAMESPACE}.json`;

  // Ensure the JSON file exists
  if (!fs.existsSync(networkJsonPath)) {
    console.log(`❌ Network JSON file not found: ${networkJsonPath}`);
    throw new Error(`Network JSON file not found: ${networkJsonPath}`);
  }

  // Read the existing JSON file
  const existingJson = JSON.parse(fs.readFileSync(networkJsonPath, 'utf8'));

  // Check if "axelar.contracts.VotingVerifier" exists in the JSON
  if (!existingJson.axelar?.contracts?.VotingVerifier) {
    console.log(`❌ No 'VotingVerifier' section found inside axelar.contracts in ${networkJsonPath}!`);
    throw new Error(`No 'VotingVerifier' section found inside axelar.contracts in ${networkJsonPath}`);
  }

  // Check if CHAIN_NAME already exists in VotingVerifier
  if (existingJson.axelar.contracts.VotingVerifier[config.CHAIN_NAME!]) {
    console.log(`❌ Chain '${config.CHAIN_NAME}' already exists in VotingVerifier! Aborting to prevent overwriting.`);
    throw new Error(`Chain '${config.CHAIN_NAME}' already exists in VotingVerifier!`);
  }

  // Make sure we have a governance address
  if (!config.GOVERNANCE_ADDRESS) {
    console.log("⚠️ GOVERNANCE_ADDRESS not set, attempting to extract it...");
    // Try to extract it from the ServiceRegistry
    try {
      config.GOVERNANCE_ADDRESS = require('child_process').execSync(
        `jq -r '.axelar.contracts.ServiceRegistry.governanceAccount' ${networkJsonPath}`, 
        { stdio: 'pipe' }
      ).toString().trim();
      
      console.log(`✅ Extracted GOVERNANCE_ADDRESS from ServiceRegistry: ${config.GOVERNANCE_ADDRESS}`);
      
      // Check if the extracted value is valid
      if (!config.GOVERNANCE_ADDRESS || config.GOVERNANCE_ADDRESS === "null" || config.GOVERNANCE_ADDRESS === "undefined") {
        // Fallback to a hard-coded value or the first validator in the validators list
        console.log("⚠️ Could not extract valid GOVERNANCE_ADDRESS, attempting to use first validator address...");
        const firstValidator = require('child_process').execSync(
          `jq -r '.validators[0].axelarAddress' ${networkJsonPath}`,
          { stdio: 'pipe' }
        ).toString().trim();
        
        if (firstValidator && firstValidator !== "null" && firstValidator !== "undefined") {
          config.GOVERNANCE_ADDRESS = firstValidator;
          console.log(`✅ Using first validator address as GOVERNANCE_ADDRESS: ${config.GOVERNANCE_ADDRESS}`);
        } else {
          throw new Error("Could not determine a valid GOVERNANCE_ADDRESS");
        }
      }
    } catch (error) {
      console.error(`Error extracting governance address: ${error}`);
      throw new Error(`Missing GOVERNANCE_ADDRESS and could not extract it automatically: ${error}`);
    }
  }

  // Handle voting threshold parsing properly
  let votingThreshold = ["6", "10"]; // Default value
  
  if (config.VOTING_THRESHOLD) {
    try {
      // If it's already an array, use it directly
      if (Array.isArray(config.VOTING_THRESHOLD)) {
        votingThreshold = config.VOTING_THRESHOLD;
      } 
      // If it's a string representation of an array
      else if (typeof config.VOTING_THRESHOLD === 'string') {
        // Check if the string starts with [ and ends with ]
        if (config.VOTING_THRESHOLD.trim().startsWith('[') && config.VOTING_THRESHOLD.trim().endsWith(']')) {
          votingThreshold = JSON.parse(config.VOTING_THRESHOLD);
        } 
        // If it's a comma-separated string like "6,10"
        else if (config.VOTING_THRESHOLD.includes(',')) {
          votingThreshold = config.VOTING_THRESHOLD.split(',').map(item => item.trim());
        }
        // If it's just a single value like "6"
        else {
          console.log(`⚠️ VOTING_THRESHOLD is a single value: ${config.VOTING_THRESHOLD}. Using ["${config.VOTING_THRESHOLD}", "10"]`);
          votingThreshold = [config.VOTING_THRESHOLD, "10"];
        }
      }
    } catch (error) {
      console.error(`Error parsing VOTING_THRESHOLD (${config.VOTING_THRESHOLD}): ${error}`);
      console.log(`⚠️ Using default VOTING_THRESHOLD: ${JSON.stringify(votingThreshold)}`);
    }
  }
  
  console.log(`✅ Using VOTING_THRESHOLD: ${JSON.stringify(votingThreshold)}`);

  // Create the new chain entry with the governance address
  const newChainEntry = {
    governanceAddress: config.GOVERNANCE_ADDRESS,
    serviceName: config.SERVICE_NAME || "validators",
    sourceGatewayAddress: config.PROXY_GATEWAY_ADDRESS,
    votingThreshold: votingThreshold,
    blockExpiry: 10,
    confirmationHeight: config.CONFIRMATION_HEIGHT ? parseInt(config.CONFIRMATION_HEIGHT) : 1,
    msgIdFormat: "hex_tx_hash_and_event_index",
    addressFormat: "eip55"
  };

  // Insert the new chain entry into axelar.contracts.VotingVerifier
  existingJson.axelar.contracts.VotingVerifier[config.CHAIN_NAME!] = newChainEntry;

  // Write the updated JSON back to file
  fs.writeFileSync(networkJsonPath, JSON.stringify(existingJson, null, 2));
  console.log(`✅ Successfully added '${config.CHAIN_NAME}' to VotingVerifier inside axelar.contracts in ${networkJsonPath}`);
  console.log(`✅ Set governanceAddress to: ${config.GOVERNANCE_ADDRESS}`);

  // Double-check that the update was successful
  try {
    const updatedJson = JSON.parse(fs.readFileSync(networkJsonPath, 'utf8'));
    const updatedValue = updatedJson.axelar.contracts.VotingVerifier[config.CHAIN_NAME!]?.governanceAddress;
    
    if (updatedValue) {
      console.log(`✅ Verified governanceAddress was set to: ${updatedValue}`);
    } else {
      console.log(`❌ Warning: Could not verify governanceAddress was set correctly!`);
    }
  } catch (error) {
    console.error(`Error verifying update: ${error}`);
  }
}

/**
 * Function to verify the transaction execution
 */
export async function verifyExecution(): Promise<void> {
  console.log("⚡ Verifying the transaction execution...");

  const jsonQuery = JSON.stringify({ chain_info: config.CHAIN_NAME });

  try {
    const { stdout } = await execAsync(`axelard q wasm contract-state smart "${config.ROUTER_ADDRESS}" '${jsonQuery}' --node "${config.AXELAR_RPC_URL}"`);
    
    // Print raw output for debugging
    console.log("🔍 Verification Output:");
    console.log(stdout);

    // Extract Gateway Address - updated regex to match the actual output format
    // Looking for the "address:" field under the "gateway:" section
    const gatewayMatch = stdout.match(/gateway:[\s\S]*?address:\s+(\S+)/m);
    const verifiedGatewayAddress = gatewayMatch ? gatewayMatch[1] : null;

    // Ensure the gateway address matches expected value
    if (verifiedGatewayAddress && verifiedGatewayAddress === config.GATEWAY_ADDRESS) {
      console.log(`✅ Verification successful! Gateway address matches: ${verifiedGatewayAddress}`);
    } else {
      console.log(`❌ Verification failed! Expected: ${config.GATEWAY_ADDRESS}, Got: ${verifiedGatewayAddress || "address not found"}`);
      throw new Error("Chain registration verification failed");
    }
  } catch (error) {
    console.error(`Error during verification: ${error}`);
    throw error;
  }
}

/**
 * Retrieve the voting verifier address
 */
export function retrieveVotingVerifierAddress(): void {
  try {
    const jsonPath = `.axelar.contracts.VotingVerifier.${config.CHAIN_NAME}.address`;
    const votingVerifierAddress = execSyncWithOutput(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -rM '${jsonPath}'`);
    config.VOTING_VERIFIER_ADDRESS = votingVerifierAddress;
    console.log(`✅ Retrieved VOTING_VERIFIER_ADDRESS: ${votingVerifierAddress}`);
  } catch (error) {
    console.error(`Error retrieving voting verifier address: ${error}`);
    throw error;
  }
}

/**
 * Execute a command synchronously and return the output
 */
function execSyncWithOutput(command: string): string {
  try {
    return require('child_process').execSync(command, { stdio: 'pipe' }).toString().trim();
  } catch (error) {
    console.error(`Error executing command: ${command}`);
    throw error;
  }
}