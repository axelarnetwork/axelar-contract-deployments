/**
 * Chain registration functions
 */

import { execSync } from 'child_process';
import { config } from '../config/environment';
import { isCustomDevnet } from '../config/network';
import { execAsync } from '../utils/exec';
import { buildJsonCmdRegister } from '../utils/json';

/**
 * Function to register a chain
 */
export async function registerChain(): Promise<void> {
  const jsonCmdRegister = buildJsonCmdRegister();

  if (isCustomDevnet()) {
    // Run the command to register the chain
    console.log("⚡ Registering the chain...");
    try {
      await execAsync(`axelard tx wasm execute "${config.ROUTER_ADDRESS}" '${jsonCmdRegister}' \
        --from amplifier \
        --gas auto \
        --gas-adjustment 2 \
        --node "${config.AXELAR_RPC_URL}" \
        --gas-prices 0.00005${config.TOKEN_DENOM} \
        --keyring-backend test \
        --chain-id "${config.NAMESPACE}"`);
    } catch (error) {
      console.error(`Error registering chain: ${error}`);
      throw error;
    }
  } else {
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
    } catch (error) {
      console.error(`Error submitting register gateway proposal: ${error}`);
      throw error;
    }
  }
}

/**
 * Run governance command to retrieve router and gateway addresses
 */
export function retrieveAddresses(): void {
  // Run the function to extract router address
  try {
    const routerAddress = execSync(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -r .axelar.contracts.Router.address`, { stdio: 'pipe' }).toString().trim();
    config.ROUTER_ADDRESS = routerAddress;
    console.log(`✅ Retrieved ROUTER_ADDRESS: ${routerAddress}`);
  
    const gatewayAddress = execSync(`cat ../axelar-chains-config/info/${config.NAMESPACE}.json | jq -r .axelar.contracts.Gateway.${config.CHAIN_NAME}.address`, { stdio: 'pipe' }).toString().trim();
    config.GATEWAY_ADDRESS = gatewayAddress;
    console.log(`✅ Retrieved GATEWAY_ADDRESS: ${gatewayAddress}`);
  } catch (error) {
    console.error(`Error retrieving addresses: ${error}`);
    throw error;
  }
}