import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';
import { config } from '../config/environment';
import { displayMessage, MessageType } from '../utils/cli-utils';
import * as yaml from 'js-yaml';

// Define interfaces for the YAML structure
interface Handler {
  type: string;
  cosmwasm_contract: string;
  chain_name: string;
  chain_rpc_url: string;
  chain_finalization: string;
  rpc_url?: string;
}

interface ConfigToml {
  handlers: Handler[];
  [key: string]: any;
}

interface HelmValues {
  config_toml: ConfigToml;
  [key: string]: any;
}

/**
 * Update the infrastructure helm values with voting verifier and RPC info
 */
export async function updateInfrastructureConfig(): Promise<void> {
  try {
    displayMessage(MessageType.INFO, "Updating infrastructure submodule configuration...");

    if (!config.CHAIN_NAME) throw new Error("CHAIN_NAME not found in configuration");
    if (!config.RPC_URL) throw new Error("RPC_URL not found in configuration");
    if (!config.VOTING_VERIFIER_ADDRESS) throw new Error("VOTING_VERIFIER_ADDRESS not found in configuration");

    const chainName = config.CHAIN_NAME;
    const rpcUrl = config.RPC_URL;
    const votingVerifierAddress = config.VOTING_VERIFIER_ADDRESS;
    const axelarMultisigAddress = config.AXELAR_MULTISIG_ADDRESS;
    const namespace = config.NAMESPACE;
    const finality = config.AMPD_FINALITY;
    let helmValuesPath: string;
    if (namespace === 'devnet-amplifier') {
      helmValuesPath = path.join(
        'infrastructure',
        'devnet',
        'apps',
        'axelar-devnet-amplifier',
        'ampd-set-2',
        'helm-values.yaml'
      );
    } else if (namespace === 'testnet' || namespace === 'stagenet') {
      helmValuesPath = path.join(
        'infrastructure',
        'testnet',
        'apps',
        `axelar-${namespace}`,
        'ampd',
        'helm-values.yaml'
      );
    } else {
      helmValuesPath = path.join(
        'infrastructure',
        'mainnet',
        'apps',
        `axelar-mainnet`,
        'ampd',
        'helm-values.yaml'
      );
    }

    const branchName = `chore/${namespace}-add-${chainName}-ampd`;

    if (!fs.existsSync('infrastructure/.git')) {
      displayMessage(MessageType.ERROR, "Not in a directory with the infrastructure submodule");
      throw new Error("Infrastructure submodule not found");
    }

    process.chdir('infrastructure');

    try {
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

      if (!fs.existsSync(helmValuesPath)) {
        displayMessage(MessageType.ERROR, `Helm values file not found at: ${helmValuesPath}`);
        throw new Error(`Helm values file not found at: ${helmValuesPath}`);
      }

      // Read the file content as string
      const fileContent = fs.readFileSync(helmValuesPath, 'utf8');
      
      // Parse the YAML only to check if we need to add handlers
      const doc = yaml.load(fileContent) as HelmValues;

      if (!doc.config_toml || !Array.isArray(doc.config_toml.handlers)) {
        throw new Error(`Malformed helm values: missing config_toml.handlers array`);
      }

      const handlers = doc.config_toml.handlers;

      const hasMsgVerifier = handlers.some(
        h => h.type === 'EvmMsgVerifier' && h.chain_name === chainName
      );

      const hasSetVerifier = handlers.some(
        h => h.type === 'EvmVerifierSetVerifier' && h.chain_name === chainName
      );

      const hasMultisigSigner = handlers.some(
        h => h.type === 'MultisigSigner' && h.chain_name === chainName
      );

      // Only modify the file if we need to add new handlers
      if (!hasMsgVerifier || !hasSetVerifier || !hasMultisigSigner) {
        // Here's the key change: We'll manually edit the file instead of parsing and dumping
        const lines = fileContent.split('\n');
        
        // Find the end of the handlers section
        let handlersEndLineIndex = -1;
        let handlersIndentation = '  ';  // Default indentation
        let inHandlersSection = false;
        
        for (let i = 0; i < lines.length; i++) {
          const line = lines[i];
          
          if (line.match(/config_toml:.*handlers:/)) {
            inHandlersSection = true;
            const match = line.match(/^(\s*)/);
            handlersIndentation = match ? match[1] + '  ' : '  ';
            continue;
          }
          
          if (line.match(/^\s*handlers:/)) {
            inHandlersSection = true;
            const match = line.match(/^(\s*)/);
            handlersIndentation = match ? match[1] + '  ' : '  ';
            continue;
          }
          
          if (inHandlersSection) {
            // Check if this line has the same or less indentation than handlers section
            // which would indicate we've moved past the handlers section
            const match = line.match(/^(\s*)/);
            const currentIndent = match ? match[1] : '';
            
            if (currentIndent.length <= handlersIndentation.length - 2 && line.trim() !== '') {
              handlersEndLineIndex = i;
              break;
            }
            
            // If we reach the end of the file, set the insertion point to the last line
            if (i === lines.length - 1) {
              handlersEndLineIndex = i + 1;
            }
          }
        }
        
        if (handlersEndLineIndex === -1) {
          throw new Error("Could not find handlers section in YAML file");
        }
        
        // Create the new handler entries with the proper indentation - REMOVED QUOTES
        const newEntries = [];
        
        if (!hasMsgVerifier) {
          newEntries.push(
            `${handlersIndentation}- type: EvmMsgVerifier`,
            `${handlersIndentation}  cosmwasm_contract: ${votingVerifierAddress}`,
            `${handlersIndentation}  chain_name: ${chainName}`,
            `${handlersIndentation}  chain_rpc_url: ${rpcUrl}`,
            `${handlersIndentation}  chain_finalization: ${finality}`
          );
          displayMessage(MessageType.SUCCESS, `Added EvmMsgVerifier handler for ${chainName}`);
        }
        
        if (!hasSetVerifier) {
          newEntries.push(
            `${handlersIndentation}- type: EvmVerifierSetVerifier`,
            `${handlersIndentation}  cosmwasm_contract: ${votingVerifierAddress}`,
            `${handlersIndentation}  chain_name: ${chainName}`,
            `${handlersIndentation}  chain_rpc_url: ${rpcUrl}`,
            `${handlersIndentation}  chain_finalization: ${finality}`
          );
          displayMessage(MessageType.SUCCESS, `Added EvmVerifierSetVerifier handler for ${chainName}`);
        }

        if (!hasMultisigSigner) {
          newEntries.push(
            `${handlersIndentation}- type: MultisigSigner`,
            `${handlersIndentation}  cosmwasm_contract: ${axelarMultisigAddress}`,
            `${handlersIndentation}  chain_name: ${chainName}`,
          );
        }
        
        // Insert the new entries at the end of the handlers section
        lines.splice(handlersEndLineIndex, 0, ...newEntries);
        
        // Write the modified file back
        fs.writeFileSync(helmValuesPath, lines.join('\n'));
        
        displayMessage(MessageType.INFO, "Committing changes...");
        execSync('git add .', { stdio: 'pipe' });
        execSync(`git commit -m "chore: add ${chainName} to ${namespace} AMPD configuration"`, { stdio: 'pipe' });

        displayMessage(MessageType.INFO, "Pushing branch to origin...");
        execSync(`git push -u origin ${branchName}`, { stdio: 'pipe' });

        const repoUrl = execSync('git remote get-url origin', { stdio: 'pipe' }).toString().trim();
        const repoPath = repoUrl.replace(/^(https:\/\/github.com\/|git@github.com:)/, '').replace(/\.git$/, '');
        const prUrl = `https://github.com/${repoPath}/compare/main...${branchName}?expand=1`;

        displayMessage(
          MessageType.SUCCESS,
          `Branch pushed to origin. Create a PR manually by visiting:\n${prUrl}`
        );
      } else {
        displayMessage(MessageType.INFO, `Handlers for ${chainName} already exist, no changes needed`);
      }

      process.chdir('..');

    } catch (gitError) {
      process.chdir('..');
      displayMessage(MessageType.ERROR, `Git operations failed: ${gitError}`);
      throw gitError;
    }

  } catch (error) {
    displayMessage(MessageType.ERROR, `Failed to update infrastructure config: ${error}`);
    throw error;
  }
}