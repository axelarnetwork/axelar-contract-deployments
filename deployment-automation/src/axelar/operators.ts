/**
 * Deploy the AxelarGasService contract using the Operators address as collector
 * @returns Promise containing the deployment output
 */
export async function deployAxelarGasService(operatorsAddress: string, deployMethod: string = 'create2'): Promise<string> {
    try {
      if (!operatorsAddress) {
        throw new Error("No operators address provided for AxelarGasService deployment");
      }
      
      console.log(`⚡ Deploying AxelarGasService with collector ${operatorsAddress}...`);
      
      // Command to deploy AxelarGasService
      const deployCmd = `node ../evm/deploy-upgradable.js -c AxelarGasService -m ${deployMethod} --args '{"collector": "${operatorsAddress}"}' --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -y`;
      
      console.log("Running deployment command:", deployCmd);
      
      // For custom devnets, we might use synchronous execution
      if (isCustomDevnet()) {
        // Execute the command
        const deployOutput = execSync(deployCmd, { stdio: 'pipe' }).toString();
        console.log("Deployment output:", deployOutput);
        
        // Check if deployment was successful
        if (deployOutput.includes("Deployment status: SUCCESS")) {
          console.log("✅ AxelarGasService deployed successfully!");
        } else if (deployOutput.includes("Deployment status: FAILED")) {
          throw new Error("AxelarGasService deployment failed, check the output for details.");
        }
        
        // Extract deployed contract address if present in the output
        const addressMatch = deployOutput.match(/Contract deployed at: (0x[a-fA-F0-9]+)/);
        if (addressMatch && addressMatch[1]) {
          config.GAS_SERVICE_ADDRESS = addressMatch[1];
          console.log(`✅ Extracted GAS_SERVICE_ADDRESS: ${config.GAS_SERVICE_ADDRESS}`);
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
          console.log("✅ AxelarGasService deployed successfully!");
          
          // Extract deployed contract address if present in the output
          const addressMatch = stdout.match(/Contract deployed at: (0x[a-fA-F0-9]+)/);
          if (addressMatch && addressMatch[1]) {
            config.GAS_SERVICE_ADDRESS = addressMatch[1];
            console.log(`✅ Extracted GAS_SERVICE_ADDRESS: ${config.GAS_SERVICE_ADDRESS}`);
          }
        } else if (stdout.includes("Deployment status: FAILED")) {
          throw new Error("AxelarGasService deployment failed, check the output for details.");
        } else if (stdout.includes("already deployed")) {
          console.log("✅ AxelarGasService is already deployed, reusing existing deployment.");
          
          // Try to extract the existing address
          const addressMatch = stdout.match(/Contract found at: (0x[a-fA-F0-9]+)/);
          if (addressMatch && addressMatch[1]) {
            config.GAS_SERVICE_ADDRESS = addressMatch[1];
            console.log(`✅ Using existing GAS_SERVICE_ADDRESS: ${config.GAS_SERVICE_ADDRESS}`);
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
  }/**
   * Operator management module
   * Handles operations related to deploying Operators contract and adding operators
   */
  
  import { execSync } from 'child_process';
  import * as fs from 'fs';
  import { config } from '../config/environment';
  import { execAsync } from '../utils/exec';
  import { isCustomDevnet } from '../config/network';
  import { displayMessage, MessageType } from '../utils/cli-utils';
  
  /**
   * Deploy the Operators contract
   * @returns Promise containing the deployment output and the deployed address
   */
  export async function deployOperatorsContract(): Promise<{output: string, address: string}> {
    try {
      console.log("⚡ Deploying Operators contract...");
      
      // Command to deploy Operators contract
      const deployCmd = `node ../evm/deploy-contract.js -c Operators -m create2 --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -y`;
      
      console.log("Running deployment command:", deployCmd);
      
      // For custom devnets, we might use synchronous execution
      if (isCustomDevnet()) {
        // Execute the command
        const deployOutput = execSync(deployCmd, { stdio: 'pipe' }).toString();
        console.log("Deployment output:", deployOutput);
        
        // Extract the operators address from the output
        const operatorsAddressMatch = deployOutput.match(new RegExp(`${config.CHAIN_NAME}\\s*\\|\\s*Operators:\\s*(0x[a-fA-F0-9]+)`));
        const operatorsAddress = operatorsAddressMatch ? operatorsAddressMatch[1] : '';
        
        if (!operatorsAddress) {
          throw new Error("Could not extract Operators contract address from deployment output");
        }
        
        // Store the operators address in the config
        config.OPERATORS_CONTRACT_ADDRESS = operatorsAddress;
        console.log(`✅ Extracted OPERATORS_CONTRACT_ADDRESS: ${operatorsAddress}`);
        
        return { output: deployOutput, address: operatorsAddress };
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
        
        // Extract the operators address from the output
        const operatorsAddressMatch = stdout.match(new RegExp(`${config.CHAIN_NAME}\\s*\\|\\s*Operators:\\s*(0x[a-fA-F0-9]+)`));
        const operatorsAddress = operatorsAddressMatch ? operatorsAddressMatch[1] : '';
        
        if (!operatorsAddress) {
          // Fallback to other address extraction methods
          const addressMatch = stdout.match(/Contract deployed at: (0x[a-fA-F0-9]+)/);
          if (addressMatch && addressMatch[1]) {
            config.OPERATORS_CONTRACT_ADDRESS = addressMatch[1];
            console.log(`✅ Extracted OPERATORS_CONTRACT_ADDRESS: ${config.OPERATORS_CONTRACT_ADDRESS}`);
            return { output: stdout, address: addressMatch[1] };
          }
          
          throw new Error("Could not extract Operators contract address from deployment output");
        }
        
        // Store the operators address in the config
        config.OPERATORS_CONTRACT_ADDRESS = operatorsAddress;
        console.log(`✅ Extracted OPERATORS_CONTRACT_ADDRESS: ${operatorsAddress}`);
        
        return { output: stdout, address: operatorsAddress };
      }
    } catch (error: any) {
      // Special handling for specific error cases
      const errorMessage = String(error);
      
      // Check if this is the "already deployed" case which might be expected in some environments
      if (errorMessage.includes("already deployed") || errorMessage.includes("already exists")) {
        console.log("✅ Operators contract is already deployed, continuing...");
        
        // Try to extract the existing address if available in the error message
        const addressMatch = errorMessage.match(/at address (0x[a-fA-F0-9]+)/);
        if (addressMatch && addressMatch[1]) {
          config.OPERATORS_CONTRACT_ADDRESS = addressMatch[1];
          console.log(`✅ Using existing OPERATORS_CONTRACT_ADDRESS: ${config.OPERATORS_CONTRACT_ADDRESS}`);
          return { output: "Already deployed", address: addressMatch[1] };
        }
        
        if (config.OPERATORS_CONTRACT_ADDRESS) {
          // Ensure the address is a string
          const address = String(config.OPERATORS_CONTRACT_ADDRESS);
          return { output: "Already deployed", address: address };
        }
        
        throw new Error("Operators contract is already deployed but could not determine its address");
      }
      
      console.error(`Error deploying Operators contract: ${error}`);
      throw error;
    }
  }
  
  /**
   * Add an operator to the Operators contract
   * @returns Promise containing the operation output
   */
  export async function addOperator(): Promise<string> {
    try {      
      console.log(`⚡ Adding operator ${config.OPERATORS_CONTRACT_ADDRESS}...`);
      
      // Command to add operator
      const addCmd = `node ../evm/operators.js --action addOperator --args ${config.OPERATORS_CONTRACT_ADDRESS} --env "${config.NAMESPACE}" -n "${config.CHAIN_NAME}" -y`;
      
      console.log("Running add operator command:", addCmd);
      
      // For custom devnets, we might use synchronous execution
      if (isCustomDevnet()) {
        // Execute the command
        const addOutput = execSync(addCmd, { stdio: 'pipe' }).toString();
        console.log("Add operator output:", addOutput);
        
        // Check if operation was successful
        if (addOutput.includes("Operation status: SUCCESS")) {
          console.log(`✅ Operator ${config.OPERATORS_CONTRACT_ADDRESS} added successfully!`);
        } else if (addOutput.includes("Operation status: FAILED")) {
          throw new Error("Adding operator failed, check the output for details.");
        }
        
        return addOutput;
      } else {
        // For non-devnet environments, use execAsync for better error handling
        const { stdout, stderr } = await execAsync(addCmd, { maxBuffer: 1024 * 1024 * 10 }); // 10MB buffer
        
        // Log the complete command output
        console.log(`\n==== COMMAND OUTPUT START ====`);
        console.log(stdout);
        if (stderr) {
          console.error(`==== STDERR OUTPUT ====`);
          console.error(stderr);
        }
        console.log(`==== COMMAND OUTPUT END ====\n`);
        
        // Check for success message
        if (stdout.includes("Operation status: SUCCESS")) {
          console.log(`✅ Operator ${config.OPERATORS_CONTRACT_ADDRESS} added successfully!`);
        } else if (stdout.includes("Operation status: FAILED")) {
          throw new Error("Adding operator failed, check the output for details.");
        } else if (stdout.includes("already an operator")) {
          console.log(`✅ ${config.OPERATORS_CONTRACT_ADDRESS} is already an operator, continuing...`);
        }
        
        // Save output to file for record keeping
        try {
          const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
          const logFilePath = `./logs/add-operator-${config.OPERATORS_CONTRACT_ADDRESS}-${timestamp}.log`;
          await fs.promises.mkdir('./logs', { recursive: true });
          await fs.promises.writeFile(logFilePath, 
            `Command: ${addCmd}\n\n` +
            `STDOUT:\n${stdout}\n\n` +
            (stderr ? `STDERR:\n${stderr}\n\n` : '') +
            `Timestamp: ${new Date().toISOString()}\n`
          );
          console.log(`📄 Command output saved to ${logFilePath}`);
        } catch (logError) {
          console.error(`Failed to save log: ${logError}`);
        }
        
        return stdout;
      }
    } catch (error: any) {
      // Special handling for specific error cases
      const errorMessage = String(error);
      
      // Check if this is the "already an operator" case which might be expected
      if (errorMessage.includes("already an operator") || errorMessage.includes("already added")) {
        console.log(`✅ Address is already an operator, continuing...`);
        return "Already an operator";
      }
      
      console.error(`Error adding operator: ${error}`);
      
      // Save error details to file
      try {
        const operatorAddress = String(config.OPERATOR_ADDRESS || "unknown");
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const errorLogPath = `./logs/add-operator-error-${config.OPERATORS_CONTRACT_ADDRESS}-${timestamp}.log`;
        await fs.promises.mkdir('./logs', { recursive: true });
        await fs.promises.writeFile(errorLogPath, 
          `Error: ${error}\n\n` +
          `Timestamp: ${new Date().toISOString()}`
        );
        console.log(`📄 Error details saved to ${errorLogPath}`);
      } catch (logError) {
        console.error(`Failed to save error log: ${logError}`);
      }
      
      throw error;
    }
  }