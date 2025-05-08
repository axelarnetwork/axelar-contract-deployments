/**
 * Deployer module for contract deployment utilities
 * Handles deployment of ConstAddressDeployer and Create3Deployer contracts
 */

import { execSync } from 'child_process';
import { config } from '../config/environment';
import { execAsync } from '../utils/exec';
import { isCustomDevnet } from '../config/network';

/**
 * Function to deploy the ConstAddressDeployer contract
 * @returns Promise containing the deployment output
 */
export async function deployConstAddrDeployer(): Promise<string> {
  try {
    console.log("⚡ Deploying ConstAddressDeployer contract...");
    
    // Command to deploy ConstAddressDeployer
    const deployCmd = `node ../evm/deploy-contract.js -c ConstAddressDeployer -m create --artifactPath ../evm/legacy/ConstAddressDeployer.json --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -y`;
    
    console.log("Running deployment command:", deployCmd);
    
    // For custom devnets, we might need different handling
    if (isCustomDevnet()) {
      // Execute the command
      const deployOutput = execSync(deployCmd, { stdio: 'pipe' }).toString();
      console.log("Deployment output:", deployOutput);
      
      // Check if deployment was successful
      if (deployOutput.includes("Deployment status: SUCCESS")) {
        console.log("✅ ConstAddressDeployer deployed successfully!");
      } else if (deployOutput.includes("Deployment status: FAILED")) {
        throw new Error("ConstAddressDeployer deployment failed, check the output for details.");
      }
      
      // Extract deployed contract address if present in the output
      const addressMatch = deployOutput.match(/Contract deployed at: (0x[a-fA-F0-9]+)/);
      if (addressMatch && addressMatch[1]) {
        config.CONST_ADDR_DEPLOYER_ADDRESS = addressMatch[1];
        console.log(`✅ Extracted CONST_ADDR_DEPLOYER_ADDRESS: ${config.CONST_ADDR_DEPLOYER_ADDRESS}`);
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
      
      // Check for success message
      if (stdout.includes("Deployment status: SUCCESS")) {
        console.log("✅ ConstAddressDeployer deployed successfully!");
        
        // Extract deployed contract address if present in the output
        const addressMatch = stdout.match(/Contract deployed at: (0x[a-fA-F0-9]+)/);
        if (addressMatch && addressMatch[1]) {
          config.CONST_ADDR_DEPLOYER_ADDRESS = addressMatch[1];
          console.log(`✅ Extracted CONST_ADDR_DEPLOYER_ADDRESS: ${config.CONST_ADDR_DEPLOYER_ADDRESS}`);
        }
      } else if (stdout.includes("Deployment status: FAILED")) {
        throw new Error("ConstAddressDeployer deployment failed, check the output for details.");
      } else if (stdout.includes("already deployed")) {
        console.log("✅ ConstAddressDeployer is already deployed, reusing existing deployment.");
        
        // Try to extract the existing address
        const addressMatch = stdout.match(/Contract found at: (0x[a-fA-F0-9]+)/);
        if (addressMatch && addressMatch[1]) {
          config.CONST_ADDR_DEPLOYER_ADDRESS = addressMatch[1];
          console.log(`✅ Using existing CONST_ADDR_DEPLOYER_ADDRESS: ${config.CONST_ADDR_DEPLOYER_ADDRESS}`);
        }
      }
      
      return stdout;
    }
  } catch (error: any) {
    // Special handling for specific error cases
    const errorMessage = String(error);
    
    // Check if this is the "already deployed" case which might be expected in some environments
    if (errorMessage.includes("already deployed") || errorMessage.includes("already exists")) {
      console.log("✅ ConstAddressDeployer is already deployed, continuing...");
      
      // Try to extract the existing address if available in the error message
      const addressMatch = errorMessage.match(/at address (0x[a-fA-F0-9]+)/);
      if (addressMatch && addressMatch[1]) {
        config.CONST_ADDR_DEPLOYER_ADDRESS = addressMatch[1];
        console.log(`✅ Using existing CONST_ADDR_DEPLOYER_ADDRESS: ${config.CONST_ADDR_DEPLOYER_ADDRESS}`);
      }
      
      return "Already deployed";
    }
    
    console.error(`Error deploying ConstAddressDeployer contract: ${error}`);
    throw error;
  }
}

/**
 * Function to deploy the Create3Deployer contract
 * @returns Promise containing the deployment output
 */
export async function deployCreate3Deployer(): Promise<string> {
  try {
    console.log("⚡ Deploying Create3Deployer contract...");
    
    // Command to deploy Create3Deployer
    const deployCmd = `node ../evm/deploy-contract.js -c Create3Deployer -m create2 --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -y`;
    
    console.log("Running deployment command:", deployCmd);
    
    // For custom devnets, we might need different handling
    if (isCustomDevnet()) {
      // Execute the command
      const deployOutput = execSync(deployCmd, { stdio: 'pipe' }).toString();
      console.log("Deployment output:", deployOutput);
      
      // Check if deployment was successful
      if (deployOutput.includes("Deployment status: SUCCESS")) {
        console.log("✅ Create3Deployer deployed successfully!");
      } else if (deployOutput.includes("Deployment status: FAILED")) {
        throw new Error("Create3Deployer deployment failed, check the output for details.");
      }
      
      // Extract deployed contract address if present in the output
      const addressMatch = deployOutput.match(/Contract deployed at: (0x[a-fA-F0-9]+)/);
      if (addressMatch && addressMatch[1]) {
        config.CREATE3_DEPLOYER_ADDRESS = addressMatch[1];
        console.log(`✅ Extracted CREATE3_DEPLOYER_ADDRESS: ${config.CREATE3_DEPLOYER_ADDRESS}`);
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
      
      // Check for success message
      if (stdout.includes("Deployment status: SUCCESS")) {
        console.log("✅ Create3Deployer deployed successfully!");
        
        // Extract deployed contract address if present in the output
        const addressMatch = stdout.match(/Contract deployed at: (0x[a-fA-F0-9]+)/);
        if (addressMatch && addressMatch[1]) {
          config.CREATE3_DEPLOYER_ADDRESS = addressMatch[1];
          console.log(`✅ Extracted CREATE3_DEPLOYER_ADDRESS: ${config.CREATE3_DEPLOYER_ADDRESS}`);
        }
      } else if (stdout.includes("Deployment status: FAILED")) {
        throw new Error("Create3Deployer deployment failed, check the output for details.");
      } else if (stdout.includes("already deployed")) {
        console.log("✅ Create3Deployer is already deployed, reusing existing deployment.");
        
        // Try to extract the existing address
        const addressMatch = stdout.match(/Contract found at: (0x[a-fA-F0-9]+)/);
        if (addressMatch && addressMatch[1]) {
          config.CREATE3_DEPLOYER_ADDRESS = addressMatch[1];
          console.log(`✅ Using existing CREATE3_DEPLOYER_ADDRESS: ${config.CREATE3_DEPLOYER_ADDRESS}`);
        }
      }
      
      return stdout;
    }
  } catch (error: any) {
    // Special handling for specific error cases
    const errorMessage = String(error);
    
    // Check if this is the "already deployed" case which might be expected in some environments
    if (errorMessage.includes("already deployed") || errorMessage.includes("already exists")) {
      console.log("✅ Create3Deployer is already deployed, continuing...");
      
      // Try to extract the existing address if available in the error message
      const addressMatch = errorMessage.match(/at address (0x[a-fA-F0-9]+)/);
      if (addressMatch && addressMatch[1]) {
        config.CREATE3_DEPLOYER_ADDRESS = addressMatch[1];
        console.log(`✅ Using existing CREATE3_DEPLOYER_ADDRESS: ${config.CREATE3_DEPLOYER_ADDRESS}`);
      }
      
      return "Already deployed";
    }
    
    console.error(`Error deploying Create3Deployer contract: ${error}`);
    throw error;
  }
}