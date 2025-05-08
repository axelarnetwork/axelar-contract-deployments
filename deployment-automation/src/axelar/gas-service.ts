/**
 * Gas Service management module
 * Handles operations related to deploying Gas Service contract
 */

import { execSync } from 'child_process';
import { config } from '../config/environment';
import { execAsync } from '../utils/exec';
import { isCustomDevnet } from '../config/network';


/**
 * Deploy the AxelarGasService contract using the Operators address as collector
 * @returns Promise containing the deployment output
 */
export async function deployAxelarGasService(): Promise<string> {
    try {
      if (!config.OPERATORS_CONTRACT_ADDRESS) {
        throw new Error("No operators address provided for AxelarGasService deployment");
      }
      
      console.log(`⚡ Deploying AxelarGasService with collector ${config.OPERATORS_CONTRACT_ADDRESS}...`);
      
      // Command to deploy AxelarGasService
      //TODO update deployment type for gas service as config
      const deployCmd = `node ../evm/deploy-upgradable.js -c AxelarGasService -m create2 --args '{"collector": "${config.OPERATORS_CONTRACT_ADDRESS}"}' --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -y`;
      
      console.log("Running deployment command:", deployCmd);
      
      // For custom devnets, we might use synchronous execution
      if (isCustomDevnet()) {
        // Execute the command
        const deployOutput = execSync(deployCmd, { stdio: 'pipe' }).toString();
        console.log("Deployment output:", deployOutput);
        
        // Extract deployed contract address from proxy address regardless of success message
        const proxyAddressMatch = deployOutput.match(/\w+\s+\|\s+Proxy for AxelarGasService:\s+(0x[a-fA-F0-9]+)/);
        if (proxyAddressMatch && proxyAddressMatch[1]) {
          config.GAS_SERVICE_ADDRESS = proxyAddressMatch[1];
          console.log(`✅ Extracted GAS_SERVICE_ADDRESS: ${config.GAS_SERVICE_ADDRESS}`);
        }
        
        // Check if deployment was successful
        if (deployOutput.includes("Deployment status: SUCCESS") || proxyAddressMatch) {
          console.log("✅ AxelarGasService deployed successfully!");
        } else if (deployOutput.includes("Deployment status: FAILED")) {
          throw new Error("AxelarGasService deployment failed, check the output for details.");
        }
        
        // If we still didn't get the address, try the old pattern as fallback
        if (!config.GAS_SERVICE_ADDRESS) {
          const addressMatch = deployOutput.match(/Contract deployed at: (0x[a-fA-F0-9]+)/);
          if (addressMatch && addressMatch[1]) {
            config.GAS_SERVICE_ADDRESS = addressMatch[1];
            console.log(`✅ Extracted GAS_SERVICE_ADDRESS: ${config.GAS_SERVICE_ADDRESS}`);
          }
        }
        
        return deployOutput;
      } else {
        // For non-devnet environments, use execAsync for better error handling
        const { stdout, stderr } = await execAsync(deployCmd, { maxBuffer: 1024 * 1024 * 10 }); // 10MB buffer
        
        // Log the complete command output
        console.log(`\n==== COMMAND OUTPUT START ====`);
        console.log(stdout);
        if (stderr) {
          console.error(`==== STDERR OUTPUT ====`);
          console.error(stderr);
        }
        console.log(`==== COMMAND OUTPUT END ====\n`);
        
        // Extract deployed contract address from proxy address regardless of success message
        const proxyAddressMatch = stdout.match(/\w+\s+\|\s+Proxy for AxelarGasService:\s+(0x[a-fA-F0-9]+)/);
        if (proxyAddressMatch && proxyAddressMatch[1]) {
          config.GAS_SERVICE_ADDRESS = proxyAddressMatch[1];
          console.log(`✅ Extracted GAS_SERVICE_ADDRESS: ${config.GAS_SERVICE_ADDRESS}`);
        }
        
        // Then continue with your success/failure checks
        if (stdout.includes("Deployment status: SUCCESS") || proxyAddressMatch) {
          console.log("✅ AxelarGasService deployed successfully!");
        } else if (stdout.includes("Deployment status: FAILED")) {
          throw new Error("AxelarGasService deployment failed, check the output for details.");
        } else if (stdout.includes("already deployed")) {
          console.log("✅ AxelarGasService is already deployed, reusing existing deployment.");
          
          // Try to extract the existing address if we didn't already get it from proxy output
          if (!config.GAS_SERVICE_ADDRESS) {
            const addressMatch = stdout.match(/Contract found at: (0x[a-fA-F0-9]+)/);
            if (addressMatch && addressMatch[1]) {
              config.GAS_SERVICE_ADDRESS = addressMatch[1];
              console.log(`✅ Using existing GAS_SERVICE_ADDRESS: ${config.GAS_SERVICE_ADDRESS}`);
            }
          }
        }
        
        return stdout;
      }
    } catch (error: any) {
      // Special handling for specific error cases
      const errorMessage = String(error);
      
      // Check if this is the "already deployed" case which might be expected in some environments
      if (errorMessage.includes("already deployed") || errorMessage.includes("already exists")) {
        console.log("✅ AxelarGasService is already deployed, continuing...");
        
        // Try to extract the existing address if available in the error message
        const addressMatch = errorMessage.match(/at address (0x[a-fA-F0-9]+)/);
        if (addressMatch && addressMatch[1]) {
          config.GAS_SERVICE_ADDRESS = addressMatch[1];
          console.log(`✅ Using existing GAS_SERVICE_ADDRESS: ${config.GAS_SERVICE_ADDRESS}`);
        }
        
        return "Already deployed";
      }
      
      console.error(`Error deploying AxelarGasService: ${error}`);
      throw error;
    }
  }