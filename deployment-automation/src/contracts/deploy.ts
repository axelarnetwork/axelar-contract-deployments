/**
 * Contract deployment functions
 */

import * as fs from 'fs';
import { WASM_DIR } from '../../constants';
import { config } from '../config/environment';
import { isCustomDevnet } from '../config/network';
import { ContractFile } from './types';
import { execAsync } from '../utils/exec';

/**
 * Function to extract SALT value from the correct checksums file
 */
export function extractSalt(contractName: string): void {
  const checkSumFile = `${WASM_DIR}/${contractName}_checksums.txt`;

  if (!fs.existsSync(checkSumFile)) {
    console.log(`❌ Checksum file not found: ${checkSumFile}`);
    throw new Error(`Checksum file not found: ${checkSumFile}`);
  }

  // Extract the correct checksum (SALT) for the contract
  const fileContent = fs.readFileSync(checkSumFile, 'utf8');
  const match = fileContent.match(new RegExp(`(\\S+)\\s+${contractName}\\.wasm`));

  if (!match || !match[1]) {
    console.log(`❌ Failed to extract SALT for ${contractName}!`);
    throw new Error(`Failed to extract SALT for ${contractName}`);
  }

  config.SALT = match[1];
  console.log(`✅ Extracted SALT: ${config.SALT}`);
}

/**
 * Function to deploy contracts
 */
export async function deployContracts(): Promise<void> {
  if (isCustomDevnet()) {    
    extractSalt("voting_verifier");

    // Run the deployment command with explicit file path
    console.log("⚡ Deploying VotingVerifier Contract...");
    console.log(`Using version: ${config.VOTING_VERIFIER_CONTRACT_VERSION}`);
    try {
      await execAsync(`node ../cosmwasm/deploy-contract.js upload-instantiate \
        -m "${config.MNEMONIC}" \
        -c "VotingVerifier" \
        -v "${config.VOTING_VERIFIER_CONTRACT_VERSION}" \
        -e "${config.NAMESPACE}" \
        -n "${config.CHAIN_NAME}" \
        --admin "${config.CONTRACT_ADMIN}" \
        -y \
        --salt "${config.SALT}"`);
    } catch (error) {
      console.error(`Error deploying VotingVerifier: ${error}`);
      throw error;
    }

    // Extract SALT for "Gateway"
    extractSalt("gateway");

    // Run the deployment command for Gateway contract with explicit file path
    console.log("⚡ Deploying Gateway Contract...");
    console.log(`Using version: ${config.GATEWAY_CONTRACT_VERSION}`);
    try {
      await execAsync(`node ../cosmwasm/deploy-contract.js upload-instantiate \
        -m "${config.MNEMONIC}" \
        -c "Gateway" \
        -e "${config.NAMESPACE}" \
        -v "${config.GATEWAY_CONTRACT_VERSION}" \
        -n "${config.CHAIN_NAME}" \
        --admin "${config.CONTRACT_ADMIN}" \
        -y \
        --salt "${config.SALT}"`);
    } catch (error) {
      console.error(`Error deploying Gateway: ${error}`);
      throw error;
    }

    // Extract SALT for "MultisigProver"
    extractSalt("multisig_prover");

    // Run the deployment command for MultisigProver contract with explicit file path
    console.log("⚡ Deploying MultisigProver Contract...");
    console.log(`Using version: ${config.MULTISIG_PROVER_CONTRACT_VERSION}`);
    try {
      await execAsync(`node ../cosmwasm/deploy-contract.js upload-instantiate \
        -m "${config.MNEMONIC}" \
        -c "MultisigProver" \
        -e "${config.NAMESPACE}" \
        -v "${config.MULTISIG_PROVER_CONTRACT_VERSION}" \
        -n "${config.CHAIN_NAME}" \
        --admin "${config.CONTRACT_ADMIN}" \
        -y \
        --salt "${config.SALT}"`);
    } catch (error) {
      console.error(`Error deploying MultisigProver: ${error}`);
      throw error;
    }
  } else {
    // Non-custom devnet logic
    try {
      console.log("⚡ Instantiate contracts...");
      console.log("⚡ Instantiate VotingVerifier Contract...");
      await execAsync(`node ./../cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin ${config.CONTRACT_ADMIN} -n ${config.CHAIN_NAME} -e ${config.NAMESPACE} -y`);
      console.log("⚡ Instantiate Gateway Contract...");
      await execAsync(`node ./../cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin ${config.CONTRACT_ADMIN} -n ${config.CHAIN_NAME} -e ${config.NAMESPACE} -y`);
      console.log("⚡ Instantiate MultisigProver Contract...");
      await execAsync(`node ./../cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin ${config.CONTRACT_ADMIN} -n ${config.CHAIN_NAME} -e ${config.NAMESPACE} -y`);
    } catch (error) {
      console.error(`Error instantiating contracts: ${error}`);
      throw error;
    }
  }
}