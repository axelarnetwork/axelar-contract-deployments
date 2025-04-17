/**
 * Command-line argument handling
 */

import { config } from '../config/environment';
import { displayMessage, MessageType } from './cli-utils';

/**
 * Parse command-line arguments and update config
 */
export function parseCommandLineArgs(): void {
  // Get all command line arguments
  const args = process.argv.slice(2);
  let isNewDeployment: boolean | null = null;
  let verifiersRegistered: boolean | null = null;
  let multisigProposalsApproved: boolean | null = null;

  // Process each argument
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    
    // Handle flags and options
    if (arg.startsWith('--')) {
      const option = arg.slice(2);
      
      switch (option) {
        case 'new-deployment':
          isNewDeployment = true;
          break;
        case 'resume-deployment':
          isNewDeployment = false;
          break;
        case 'verifiers-registered':
          verifiersRegistered = true;
          break;
        case 'no-verifiers-registered':
          verifiersRegistered = false;
          break;
        case 'proposals-approved':
          multisigProposalsApproved = true;
          break;
        case 'no-proposals-approved':
          multisigProposalsApproved = false;
          break;
        case 'force-gateway-deployment':
          // This flag is just checked directly in the code
          break;
        case 'continue-on-error':
          // This flag is just checked directly in the code
          break;
        case 'namespace':
          if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
            config.NAMESPACE = args[++i];
          }
          break;
        case 'chain-name':
          if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
            config.CHAIN_NAME = args[++i];
          }
          break;
        case 'chain-id':
          if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
            config.CHAIN_ID = args[++i];
          }
          break;
        case 'token-symbol':
          if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
            config.TOKEN_SYMBOL = args[++i];
          }
          break;
        case 'gas-limit':
          if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
            config.GAS_LIMIT = args[++i];
          }
          break;
        case 'rpc-url':
          if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
            config.RPC_URL = args[++i];
          }
          break;
        case 'axelar-rpc-url':
          if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
            config.AXELAR_RPC_URL = args[++i];
          }
          break;
        case 'version':
          if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
            config.CONTRACT_VERSION = args[++i];
          }
          break;
        case 'help':
          printHelp();
          process.exit(0);
          break;
        default:
          displayMessage(MessageType.WARNING, `Unknown option: ${option}`);
      }
    }
  }

  // Store parsed flags in config for later use
  if (isNewDeployment !== null) {
    config.IS_NEW_DEPLOYMENT = isNewDeployment;
  }
  
  if (verifiersRegistered !== null) {
    config.VERIFIERS_REGISTERED = verifiersRegistered;
  }
  
  if (multisigProposalsApproved !== null) {
    config.MULTISIG_PROPOSALS_APPROVED = multisigProposalsApproved;
  }
}

/**
 * Print help information
 */
export function printHelp(): void {
  console.log(`
Axelar Deployment Tool

Usage:
  npm start -- [options]

Main Options:
  --new-deployment                Start a new deployment
  --resume-deployment             Resume an existing deployment
  
Resume Options:
  --verifiers-registered          Indicate verifiers have registered support
  --no-verifiers-registered       Indicate verifiers have not registered support
  --proposals-approved            Indicate multisig proposals have been approved
  --no-proposals-approved         Indicate multisig proposals have not been approved
  --force-gateway-deployment      Try to deploy gateway even if earlier steps fail
  --continue-on-error             Continue execution despite errors

Configuration Options:
  --namespace <value>             Set the network namespace
  --chain-name <value>            Set the chain name
  --chain-id <value>              Set the chain ID
  --token-symbol <value>          Set the token symbol
  --gas-limit <value>             Set the gas limit
  --rpc-url <value>               Set the RPC URL
  --axelar-rpc-url <value>        Set the Axelar RPC URL
  --version <value>               Set the contract version
  --help                          Display this help information

Examples:
  # Start a new deployment
  npm start -- --new-deployment
  
  # Resume after initial deployment (verifiers need to register)
  npm start -- --resume-deployment --no-verifiers-registered
  
  # Resume after verifiers registered (proposals not yet approved - resubmit proposals)
  npm start -- --resume-deployment --verifiers-registered --no-proposals-approved
  
  # Resume after proposals approved (final stage)
  npm start -- --resume-deployment --verifiers-registered --proposals-approved
  
  # Force gateway deployment despite errors
  npm start -- --resume-deployment --verifiers-registered --proposals-approved --force-gateway-deployment
`);
}