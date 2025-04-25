/**
 * Update EDS configuration with new chain data
 */

import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';
import { config } from '../config/environment';
import { displayMessage, MessageType } from '../utils/cli-utils';

/**
 * Update the EDS TOML and tfvars config with new chain information
 */
export async function updateEdsConfig(): Promise<void> {
  try {
    displayMessage(MessageType.INFO, "Updating EDS configuration...");

    if (!config.CHAIN_NAME) throw new Error("CHAIN_NAME not found in configuration");
    if (!config.RPC_URL) throw new Error("RPC_URL not found in configuration");
    if (!config.PROXY_GATEWAY_ADDRESS) throw new Error("PROXY_GATEWAY_ADDRESS not found in configuration");
    if (!config.GAS_SERVICE_ADDRESS) throw new Error("GAS_SERVICE_ADDRESS not found in configuration");
    if (!config.OPERATORS_CONTRACT_ADDRESS) throw new Error("OPERATORS_CONTRACT_ADDRESS not found in configuration");
    if (!config.CHAIN_ID) throw new Error("CHAIN_ID not found in configuration");
    if (!config.GATEWAY_ADDRESS) throw new Error("GATEWAY_ADDRESS not found in configuration");
    if (!config.MULTISIG_PROVER_ADDRESS) throw new Error("MULTISIG_PROVER_ADDRESS not found in configuration");
    if (!config.VOTING_VERIFIER_ADDRESS) throw new Error("VOTING_VERIFIER_ADDRESS not found in configuration");
    if (!config.NAMESPACE) throw new Error("NAMESPACE not found in configuration");

    const chainName = config.CHAIN_NAME;
    const namespace = config.NAMESPACE;
    const branchName = `chore/${namespace}-add-${chainName}-eds`;

    // Verify submodule is initialized
    if (!fs.existsSync('axelar-eds/.git')) {
      displayMessage(MessageType.ERROR, "EDS submodule not properly initialized");
      throw new Error("EDS submodule not properly initialized");
    }

    // Change to submodule directory
    process.chdir('axelar-eds');

    try {
      // Git operations
      displayMessage(MessageType.INFO, "Checking out main branch and pulling latest changes...");
      execSync('git checkout main', { stdio: 'pipe' });
      execSync('git pull origin main', { stdio: 'pipe' });

      const branchExists = execSync(`git branch --list ${branchName}`, { stdio: 'pipe' }).toString().trim().length > 0;
      if (branchExists) {
        displayMessage(MessageType.WARNING, `Branch '${branchName}' already exists, checking it out...`);
        execSync(`git checkout ${branchName}`, { stdio: 'pipe' });
        displayMessage(MessageType.INFO, "Merging latest changes from main...");
        execSync('git merge main', { stdio: 'pipe' });
      } else {
        displayMessage(MessageType.INFO, `Creating and checking out new branch: ${branchName}`);
        execSync(`git checkout -b ${branchName}`, { stdio: 'pipe' });
      }

      // Update configuration files
      await updateEdsTomlConfig(chainName, namespace);
      await updateEdsTfvarsConfig(chainName, namespace);

      // Check if there are changes to commit
      const hasChanges = execSync('git status --porcelain', { stdio: 'pipe' }).toString().trim().length > 0;
      
      if (!hasChanges) {
        displayMessage(MessageType.WARNING, "No changes to commit");
        process.chdir('..');
        return;
      }
      
      // Commit and push changes
      displayMessage(MessageType.INFO, "Committing changes...");
      execSync('git add .', { stdio: 'pipe' });
      execSync(`git commit -m "chore: add ${chainName} to ${namespace} EDS configuration"`, { stdio: 'pipe' });

      displayMessage(MessageType.INFO, "Pushing branch to origin...");
      execSync(`git push -u origin ${branchName}`, { stdio: 'pipe' });

      const repoUrl = execSync('git remote get-url origin', { stdio: 'pipe' }).toString().trim();
      const repoPath = repoUrl.replace(/^(https:\/\/github.com\/|git@github.com:)/, '').replace(/\.git$/, '');
      const prUrl = `https://github.com/${repoPath}/compare/main...${branchName}?expand=1`;

      displayMessage(
        MessageType.SUCCESS,
        `Branch pushed to origin. Create a PR manually by visiting:\n${prUrl}`
      );
      
      // Return to original directory
      process.chdir('..');

    } catch (gitError) {
      process.chdir('..');
      displayMessage(MessageType.ERROR, `Git operations failed: ${gitError}`);
      throw gitError;
    }
  } catch (error) {
    displayMessage(MessageType.ERROR, `Failed to update EDS config: ${error}`);
    throw error;
  }
}

/**
 * Update the EDS TOML config with new chain information
 */
async function updateEdsTomlConfig(chainName: string, namespace: string): Promise<void> {
  try {
    displayMessage(MessageType.INFO, "Updating EDS TOML configuration...");

    const gasLimit = config.GAS_LIMIT; // Default to 14000000 if not provided
    const rpcUrl = config.RPC_URL;
    const gatewayAddress = config.PROXY_GATEWAY_ADDRESS;
    const gasServiceAddress = config.GAS_SERVICE_ADDRESS;
    const operatorsAddress = config.OPERATORS_CONTRACT_ADDRESS;
    const chainId = config.CHAIN_ID;
    const axelarGatewayAddress = config.GATEWAY_ADDRESS;
    const axelarProverAddress = config.MULTISIG_PROVER_ADDRESS;
    const axelarVerifierAddress = config.VOTING_VERIFIER_ADDRESS;

    const tomlFilePath = path.join(
      'infrastructure',
      'k8s',
      'microservices',
      'eds',
      'config',
      `${namespace}.amp.toml`
    );

    if (!fs.existsSync(tomlFilePath)) {
      displayMessage(MessageType.ERROR, `TOML file not found at: ${tomlFilePath}`);
      throw new Error(`TOML file not found at: ${tomlFilePath}`);
    }

    // Read the file content as string
    let fileContent = fs.readFileSync(tomlFilePath, 'utf8');
    
    // Check if the chain already exists in the TOML as an EVM section
    let chainExists = fileContent.includes(`chain-name                                = "${chainName}"`);
    
    if (!chainExists) {
      // Create new EVM section
      const newEvmSection = `
[[evm]]
chain-name                                = "${chainName}"
rpc-params                                = { request_timeout = "30s" }
rpc-url                                   = [
  "${rpcUrl}",
]
confirmation-height                       = 1
latest-block-offset                       = 1
max-block-depth-offset                    = 50000
gateway-contract-address                  = "${gatewayAddress}"
gas-service-contract-address              = "${gasServiceAddress}"
operators-contract-address                = "${operatorsAddress}"
interchain-token-service-contract-address = "0x0FCb262571be50815627C16Eca1f5F3D342FF5a5" # PLACEHOLDER
`;

      // Find the proper insertion point - after the last [[evm]] section
      const lines = fileContent.split('\n');
      let lastEvmIndex = -1;
      
      for (let i = 0; i < lines.length; i++) {
        if (lines[i].trim() === '[[evm]]') {
          lastEvmIndex = i;
        }
      }
      
      if (lastEvmIndex === -1) {
        displayMessage(MessageType.ERROR, "Could not find any [[evm]] sections in the TOML file");
        throw new Error("Could not find any [[evm]] sections in the TOML file");
      }
      
      // Find the end of the last evm section by looking for the next section
      let sectionEndIndex = -1;
      for (let i = lastEvmIndex + 1; i < lines.length; i++) {
        if (lines[i].trim().startsWith('[[') && !lines[i].trim().startsWith('[[evm')) {
          sectionEndIndex = i;
          break;
        }
      }
      
      // If no next section was found, insert at the end of the content
      if (sectionEndIndex === -1) {
        // Find the relay section for this chain if it exists
        const relayRegex = new RegExp(`\\[\\[relay\\]\\][\\s\\S]*?chain-name\\s*=\\s*"${chainName}"`, 'm');
        const hasRelay = relayRegex.test(fileContent);
        
        if (hasRelay) {
          // Insert before the first relay section
          const firstRelayIndex = fileContent.indexOf('[[relay]]');
          if (firstRelayIndex !== -1) {
            sectionEndIndex = lines.findIndex((line, index) => 
              line.trim() === '[[relay]]' && 
              fileContent.indexOf('[[relay]]') === fileContent.indexOf(line, index)
            );
          } else {
            sectionEndIndex = lines.length;
          }
        } else {
          sectionEndIndex = lines.length;
        }
      }
      
      // Insert the new EVM section before the next section
      lines.splice(sectionEndIndex, 0, newEvmSection);
      
      // Write the modified content back to the file
      fileContent = lines.join('\n');
      fs.writeFileSync(tomlFilePath, fileContent);
      
      displayMessage(MessageType.SUCCESS, `Added new [[evm]] section for ${chainName} to EDS config`);
    } else {
      displayMessage(MessageType.WARNING, `Chain ${chainName} EVM section already exists in EDS config`);
    }
    
    // Now check if a relay section exists for this chain
    const relayRegex = new RegExp(`\\[\\[relay\\]\\][\\s\\S]*?chain-name\\s*=\\s*"${chainName}"`, 'm');
    const hasRelay = relayRegex.test(fileContent);
    
    if (!hasRelay) {
      displayMessage(MessageType.INFO, `Adding relay configuration for ${chainName}...`);
      
      // Create the relay section
      const relaySection = `
[[relay]]
chain-name                      = "${chainName}"
chain-id                        = ${chainId}
wallet-source                   = "awskms"
pricing-mode                    = "eip1559"
gas-limit                       = ${gasLimit}
estimated-gas-limit-multiplier  = 1.3
stop-at-balance                 = "0.009eth"
notifications-enabled           = true
retry-config                    = { max-retries = 0 }

[relay.fund-config]
amplifier_relayer           = { amount = "1eth", fund_at_balance = "4eth" }
amplifier_refunder_relayer  = { amount = "0.2eth", fund_at_balance = "0.1eth" }`;

      // Find specific sections to insert the relay configuration after
      const content = fs.readFileSync(tomlFilePath, 'utf8');
      
      // Try to find the relay section
      const relayBlockStart = content.indexOf('[[relay]]');
      
      if (relayBlockStart !== -1) {
        // Find the last relay block
        let lastRelayBlockEnd = -1;
        let position = relayBlockStart;
        
        // Use a regex to find all relay blocks including their config
        const relayBlockPattern = /\[\[relay\]\][\s\S]*?\[relay\.fund-config\][\s\S]*?amplifier_refunder_relayer[^\n]*\n/g;
        let match;
        
        while ((match = relayBlockPattern.exec(content)) !== null) {
          lastRelayBlockEnd = match.index + match[0].length;
        }
        
        if (lastRelayBlockEnd !== -1) {
          // We found the end of the last relay block
          const beforeInsert = content.substring(0, lastRelayBlockEnd);
          const afterInsert = content.substring(lastRelayBlockEnd);
          
          // Construct the new content with the relay section inserted in the correct place
          const newContent = beforeInsert + '\n' + relaySection + '\n' + afterInsert;
          
          fs.writeFileSync(tomlFilePath, newContent);
          displayMessage(MessageType.SUCCESS, `Added relay configuration for ${chainName} to EDS config`);
        } else {
          // Fallback if we can't find the end properly
          displayMessage(MessageType.WARNING, "Could not precisely locate the end of relay blocks. Adding to the end of the file.");
          fs.appendFileSync(tomlFilePath, '\n\n' + relaySection + '\n');
        }
      } else {
        // No relay sections yet
        // Find where to insert the first relay section - after EVM sections but before other major sections
        
        // Identify the first non-EVM section
        const evmSections = content.match(/\[\[evm\]\]/g) || [];
        if (evmSections.length > 0) {
          // Find where EVM sections end
          const sections = content.match(/\[\[[^\]]+\]\]/g) || [];
          let insertPoint = content.length;
          
          for (let i = 0; i < sections.length; i++) {
            if (sections[i] !== "[[evm]]") {
              // This is a non-EVM section
              const sectionIndex = content.indexOf(sections[i]);
              if (sectionIndex > content.lastIndexOf("[[evm]]")) {
                // This section is after the last EVM section
                insertPoint = sectionIndex;
                break;
              }
            }
          }
          
          // Insert the relay section at the identified point
          const beforeInsert = content.substring(0, insertPoint);
          const afterInsert = content.substring(insertPoint);
          
          fs.writeFileSync(tomlFilePath, beforeInsert + '\n\n' + relaySection + '\n\n' + afterInsert);
          displayMessage(MessageType.SUCCESS, `Added relay configuration for ${chainName} to EDS config`);
        } else {
          // No EVM sections either? Just append to the end
          fs.appendFileSync(tomlFilePath, '\n\n' + relaySection + '\n');
          displayMessage(MessageType.SUCCESS, `Added relay configuration for ${chainName} to EDS config`);
        }
      }
    } else {
      displayMessage(MessageType.WARNING, `Relay configuration for ${chainName} already exists in EDS config`);
    }
    
    // Now check and add the amplifier_connections section
    const amplifierConnectionsRegex = new RegExp(`\\[\\[amplifier_connections\\]\\][\\s\\S]*?chain_name\\s*=\\s*"${chainName}"`, 'm');
    const hasAmplifierConnections = amplifierConnectionsRegex.test(fileContent);
    
    if (!hasAmplifierConnections) {
      displayMessage(MessageType.INFO, `Adding amplifier_connections for ${chainName}...`);
      
      // Create amplifier connections section
      const amplifierConnectionsSection = `
[[amplifier_connections]]
chain_name  = "${chainName}"

[amplifier_connections.contracts]
gateway     = "${axelarGatewayAddress}"
prover      = "${axelarProverAddress}"
verifier    = "${axelarVerifierAddress}"`;

      // Find where to insert the amplifier_connections section
      const content = fs.readFileSync(tomlFilePath, 'utf8');
      
      // Look for the section markers
      const internalChainsMarker = "### BEGIN CONTRACTS FOR INTERNALLY HOSTED CHAINS ###";
      const endInternalChainsMarker = "### END CONTRACTS FOR INTERNALLY HOSTED CHAINS ###";
      const teamTestingMarker = "### BEGIN CONTRACTS FOR CHAINS USED BY TEAM FOR TESTING ###";
      const externalChainsMarker = "### BEGIN CONTRACTS FOR EXTERNALLY HOSTED CHAINS ###";
      
      // Check if the markers exist
      if (content.includes(internalChainsMarker)) {
        // Find where to insert - right after the marker
        const markerIndex = content.indexOf(internalChainsMarker) + internalChainsMarker.length;
        
        // Insert the amplifier_connections section
        const beforeInsert = content.substring(0, markerIndex);
        const afterInsert = content.substring(markerIndex);
        
        const newContent = beforeInsert + '\n' + amplifierConnectionsSection + '\n\n' + afterInsert;
        fs.writeFileSync(tomlFilePath, newContent);
        displayMessage(MessageType.SUCCESS, `Added amplifier_connections for ${chainName} to EDS config`);
      } else if (content.includes(endInternalChainsMarker)) {
        // Find where to insert - right before the end marker
        const markerIndex = content.indexOf(endInternalChainsMarker);
        
        // Insert the amplifier_connections section
        const beforeInsert = content.substring(0, markerIndex);
        const afterInsert = content.substring(markerIndex);
        
        const newContent = beforeInsert + amplifierConnectionsSection + '\n\n' + afterInsert;
        fs.writeFileSync(tomlFilePath, newContent);
        displayMessage(MessageType.SUCCESS, `Added amplifier_connections for ${chainName} to EDS config`);
      } else if (content.includes(teamTestingMarker)) {
        // Insert before the team testing marker
        const markerIndex = content.indexOf(teamTestingMarker);
        
        // Check if we need to add the internal chains marker
        const insertContent = 
          "### BEGIN CONTRACTS FOR INTERNALLY HOSTED CHAINS ###\n" + 
          amplifierConnectionsSection + 
          "\n### END CONTRACTS FOR INTERNALLY HOSTED CHAINS ###\n\n";
        
        const beforeInsert = content.substring(0, markerIndex);
        const afterInsert = content.substring(markerIndex);
        
        const newContent = beforeInsert + insertContent + afterInsert;
        fs.writeFileSync(tomlFilePath, newContent);
        displayMessage(MessageType.SUCCESS, `Added amplifier_connections for ${chainName} to EDS config`);
      } else if (content.includes(externalChainsMarker)) {
        // Insert before the external chains marker
        const markerIndex = content.indexOf(externalChainsMarker);
        
        // Check if we need to add the internal chains marker
        const insertContent = 
          "### BEGIN CONTRACTS FOR INTERNALLY HOSTED CHAINS ###\n" + 
          amplifierConnectionsSection + 
          "\n### END CONTRACTS FOR INTERNALLY HOSTED CHAINS ###\n\n";
        
        const beforeInsert = content.substring(0, markerIndex);
        const afterInsert = content.substring(markerIndex);
        
        const newContent = beforeInsert + insertContent + afterInsert;
        fs.writeFileSync(tomlFilePath, newContent);
        displayMessage(MessageType.SUCCESS, `Added amplifier_connections for ${chainName} to EDS config`);
      } else {
        // No markers found - look for existing amplifier_connections sections
        const amplifierConnectionsPattern = /\[\[amplifier_connections\]\]/g;
        const matches = [...content.matchAll(amplifierConnectionsPattern)];
        
        if (matches.length > 0) {
          // Find the last amplifier_connections section
          const lastMatch = matches[matches.length - 1];
          const lastMatchIndex = lastMatch.index || 0; // Default to 0 if index is undefined
          
          // Find the end of this section
          const nextSectionRegex = /\[\[(?!amplifier_connections)[^\]]+\]\]/;
          const restOfContent = content.substring(lastMatchIndex);
          const nextSectionMatch = restOfContent.match(nextSectionRegex);
          
          if (nextSectionMatch && nextSectionMatch.index !== undefined) {
            // Insert before the next section
            const insertIndex = lastMatchIndex + nextSectionMatch.index;
            const beforeInsert = content.substring(0, insertIndex);
            const afterInsert = content.substring(insertIndex);
            
            const newContent = beforeInsert + '\n\n' + amplifierConnectionsSection + '\n\n' + afterInsert;
            fs.writeFileSync(tomlFilePath, newContent);
          } else {
            // Append to the end of the file
            fs.appendFileSync(tomlFilePath, '\n\n' + amplifierConnectionsSection + '\n');
          }
        } else {
          // No amplifier_connections sections, add at the end
          fs.appendFileSync(tomlFilePath, '\n\n' + amplifierConnectionsSection + '\n');
        }
        
        displayMessage(MessageType.SUCCESS, `Added amplifier_connections for ${chainName} to EDS config`);
      }
    } else {
      displayMessage(MessageType.WARNING, `Amplifier connections for ${chainName} already exists in EDS config`);
    }

  } catch (error) {
    displayMessage(MessageType.ERROR, `Failed to update EDS TOML config: ${error}`);
    throw error;
  }
}

/**
 * Updates a single tfvars file with the new chain information
 */
async function updateSingleTfvarsFile(filePath: string, chainName: string): Promise<void> {
  if (!fs.existsSync(filePath)) {
    displayMessage(MessageType.ERROR, `Tfvars file not found at: ${filePath}`);
    throw new Error(`Tfvars file not found at: ${filePath}`);
  }

  let content = fs.readFileSync(filePath, 'utf8');

  // Find the amplifier_evm_chains line
  const chainsRegex = /amplifier_evm_chains\s*=\s*\[(.*?)\]/s;
  const match = content.match(chainsRegex);

  if (match) {
    // Get existing chains
    const existingChains = match[1]
      .split(',')
      .map(chain => chain.trim())
      .filter(chain => chain.length > 0)
      .map(chain => chain.replace(/"/g, ''));

    // Check if chain already exists
    if (!existingChains.includes(chainName)) {
      // Add new chain to the list
      const updatedChains = [...existingChains, chainName]
        .map(chain => `"${chain}"`)
        .join(', ');

      // Update the content
      const updatedContent = content.replace(
        chainsRegex,
        `amplifier_evm_chains = [${updatedChains}]`
      );

      // Write back to file
      fs.writeFileSync(filePath, updatedContent);
      displayMessage(
        MessageType.SUCCESS,
        `Added ${chainName} to amplifier_evm_chains in ${filePath}`
      );
    } else {
      displayMessage(
        MessageType.WARNING,
        `Chain ${chainName} already exists in ${filePath}`
      );
    }
  } else {
    // If amplifier_evm_chains doesn't exist, create it
    const newChainConfig = `\namplifier_evm_chains = ["${chainName}"]\n`;
    fs.appendFileSync(filePath, newChainConfig);
    displayMessage(
      MessageType.SUCCESS,
      `Created amplifier_evm_chains with ${chainName} in ${filePath}`
    );
  }
}

/**
 * Update the EDS tfvars config with new chain information
 */
async function updateEdsTfvarsConfig(chainName: string, namespace: string): Promise<void> {
  try {
    displayMessage(MessageType.INFO, "Updating EDS tfvars configuration...");

    // Define paths for both tfvars files
    const amplifierTfvarsPath = path.join(
      'infrastructure',
      'terraform',
      'microservices',
      'amplifier_deployment',
      `${namespace}.tfvars`
    );

    const monitoringTfvarsPath = path.join(
      'infrastructure',
      'terraform',
      'microservices',
      'monitoring',
      `${namespace}.tfvars`
    );

    // Update both tfvars files
    await updateSingleTfvarsFile(amplifierTfvarsPath, chainName);

    if (namespace !== 'devnet-amplifier') {
      await updateSingleTfvarsFile(monitoringTfvarsPath, chainName);
    }

  } catch (error) {
    displayMessage(MessageType.ERROR, `Failed to update EDS tfvars config: ${error}`);
    throw error;
  }
}