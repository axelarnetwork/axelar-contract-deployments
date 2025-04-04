/**
 * Gateway deployment and configuration
 */

import * as fs from 'fs';
import { execSync } from 'child_process';
import { config } from '../config/environment';
import { execAsync } from '../utils/exec';
import { buildJsonCmdRegister } from '../utils/json';
import { isCustomDevnet } from '../config/network';


/**
 * Function to deploy gateway contract
 */
export async function deployGatewayContract(): Promise<string> {
    try {
      // First run in predictOnly mode to get the predicted address
      if (!isCustomDevnet()) {
        const predictCmd = `node ../evm/deploy-amplifier-gateway.js --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -m "${config.DEPLOYMENT_TYPE}" --minimumRotationDelay "${config.MINIMUM_ROTATION_DELAY}" -p "${config.TARGET_CHAIN_PRIVATE_KEY}" --predictOnly`;
      
        console.log("Running prediction command:", predictCmd);
        const predictOutput = execSync(predictCmd, { stdio: 'pipe' }).toString();
        console.log("Prediction output:", predictOutput);
        
        // Specifically check for the address mismatch warning
        const addressMismatchRegex = /Predicted address\s+(0x[a-fA-F0-9]+)\s+does not match existing deployment\s+(0x[a-fA-F0-9]+)/;
        const mismatchMatch = predictOutput.match(addressMismatchRegex);
        
        if (mismatchMatch) {
            const predictedAddress = mismatchMatch[1];
            const existingAddress = mismatchMatch[2];
            
            console.error(`❌ Address mismatch detected!`);
            console.error(`   Predicted: ${predictedAddress}`);
            console.error(`   Existing:  ${existingAddress}`);
            console.error("For non-custom devnets, this is a critical error. Please check the deployer, salt, args, or contract bytecode.");
            throw new Error("Gateway address mismatch detected. Deploy aborted.");
        }
      }
      
      // For custom devnets or if no warnings, proceed with actual deployment
      // Add -y flag to auto-confirm the deployment
      const deployCmd = `node ../evm/deploy-amplifier-gateway.js --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -m "${config.DEPLOYMENT_TYPE}" --minimumRotationDelay "${config.MINIMUM_ROTATION_DELAY}" -p "${config.TARGET_CHAIN_PRIVATE_KEY}" -y`;
      
      console.log("Running deployment command:", deployCmd);
      const deployOutput = execSync(deployCmd, { stdio: 'pipe' }).toString();
      console.log("Deployment output:", deployOutput);
      
      // Check if deployment was successful
      if (deployOutput.includes("Deployment status: SUCCESS")) {
        console.log("✅ Gateway deployed successfully!");
      } else if (deployOutput.includes("Deployment status: FAILED")) {
        throw new Error("Gateway deployment failed, check the output for details.");
      }
      
      return deployOutput;
    } catch (error) {
      console.error(`Error deploying gateway contract: ${error}`);
      throw error;
    }
  }

/**
 * Function to extract the Predicted Gateway Proxy Address
 */
export function extractProxyGatewayAddress(output: string): void {
  const match = output.match(/Predicted gateway proxy address: (0x[a-fA-F0-9]+)/);
  
  if (match && match[1]) {
    config.PROXY_GATEWAY_ADDRESS = match[1];
    console.log(`✅ Extracted and set PROXY_GATEWAY_ADDRESS: ${config.PROXY_GATEWAY_ADDRESS}`);
  } else {
    console.log("❌ Could not extract Predicted Gateway Proxy Address!");
    throw new Error("Could not extract Predicted Gateway Proxy Address");
  }
}

/**
 * Extract ROUTER_ADDRESS from the namespace JSON file
 */
export function extractRouterAddress(): void {
  const routerFile = `../axelar-chains-config/info/${config.NAMESPACE}.json`;

  if (!fs.existsSync(routerFile)) {
    console.log(`❌ Router config file not found: ${routerFile}`);
    throw new Error(`Router config file not found: ${routerFile}`);
  }

  const jsonContent = JSON.parse(fs.readFileSync(routerFile, 'utf8'));
  const routerAddress = jsonContent?.axelar?.contracts?.Router?.address;
  
  if (!routerAddress) {
    console.log("❌ Could not extract ROUTER_ADDRESS!");
    throw new Error("Could not extract ROUTER_ADDRESS");
  }

  config.ROUTER_ADDRESS = routerAddress;
  console.log(`✅ Extracted ROUTER_ADDRESS: ${config.ROUTER_ADDRESS}`);
}

/**
 * Extract GATEWAY_ADDRESS for the specified chain
 */
export function extractGatewayAddress(): void {
  const gatewayFile = `../axelar-chains-config/info/${config.NAMESPACE}.json`;

  if (!fs.existsSync(gatewayFile)) {
    console.log(`❌ Gateway config file not found: ${gatewayFile}`);
    throw new Error(`Gateway config file not found: ${gatewayFile}`);
  }

  const jsonContent = JSON.parse(fs.readFileSync(gatewayFile, 'utf8'));
  const gatewayAddress = jsonContent?.axelar?.contracts?.Gateway?.[config.CHAIN_NAME!]?.address;

  if (!gatewayAddress) {
    console.log(`❌ Could not extract GATEWAY_ADDRESS for ${config.CHAIN_NAME}!`);
    throw new Error(`Could not extract GATEWAY_ADDRESS for ${config.CHAIN_NAME}`);
  }

  config.GATEWAY_ADDRESS = gatewayAddress;
  console.log(`✅ Extracted GATEWAY_ADDRESS: ${config.GATEWAY_ADDRESS}`);
}

/**
 * Register a chain with the router
 */
export async function registerChainWithRouter(): Promise<void> {
  const jsonCmdRegister = buildJsonCmdRegister();
  
  console.log("⚡ Registering the chain...");
  
  try {
    await execAsync(`axelard tx wasm execute "${config.ROUTER_ADDRESS}" '${jsonCmdRegister}' \
      --from ${config.WALLET_ADDRESS || 'amplifier'} \
      --gas auto \
      --gas-adjustment 2 \
      --node "${config.AXELAR_RPC_URL}" \
      --gas-prices 0.00005${config.TOKEN_DENOM} \
      --keyring-backend test \
      --chain-id "${config.NAMESPACE}"`);
      
    console.log(`✅ Chain ${config.CHAIN_NAME} successfully registered with router`);
  } catch (error) {
    console.error(`Error registering chain: ${error}`);
    throw error;
  }
}

/**
 * Submit a proposal to register a chain with the router
 */
export async function submitChainRegistrationProposal(): Promise<void> {
  const jsonCmdRegister = buildJsonCmdRegister();
  
  try {
    if (config.NAMESPACE === "devnet-amplifier") {
      await execAsync(`node ../cosmwasm/submit-proposal.js execute \
        -c Router \
        -t "Register Gateway for ${config.CHAIN_NAME}" \
        -d "Register Gateway address for ${config.CHAIN_NAME} at Router contract" \
        --runAs ${config.RUN_AS_ACCOUNT} \
        --deposit ${config.DEPOSIT_VALUE} \
        --msg '${jsonCmdRegister}'`);
    } else {
      await execAsync(`node ../cosmwasm/submit-proposal.js execute \
        -c Router \
        -t "Register Gateway for ${config.CHAIN_NAME}" \
        -d "Register Gateway address for ${config.CHAIN_NAME} at Router contract" \
        --deposit ${config.DEPOSIT_VALUE} \
        --msg '${jsonCmdRegister}'`);
    }
    
    console.log(`✅ Proposal submitted to register chain ${config.CHAIN_NAME} with router`);
  } catch (error) {
    console.error(`Error submitting register gateway proposal: ${error}`);
    throw error;
  }
}