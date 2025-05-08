/**
 * Environment configuration interfaces and base config object
 */

/**
 * Interface for environment configuration
 */
export interface EnvironmentConfig {
  NAMESPACE: string;
  CHAIN_NAME?: string;
  CHAIN_ID?: string;
  TOKEN_SYMBOL?: string;
  GAS_LIMIT?: string;
  PRIVATE_KEY?: string;
  RPC_URL?: string;
  AXELAR_RPC_URL?: string;
  MNEMONIC?: string;
  GOVERNANCE_ADDRESS?: string;
  ADMIN_ADDRESS?: string;
  SERVICE_NAME?: string;
  VOTING_THRESHOLD?: string;
  SIGNING_THRESHOLD?: string;
  CONFIRMATION_HEIGHT?: string;
  MINIMUM_ROTATION_DELAY?: string;
  DEPLOYMENT_TYPE?: string;
  DEPLOYER?: string;
  CONTRACT_ADMIN?: string;
  PROVER_ADMIN?: string;
  DEPOSIT_VALUE?: string;
  REWARD_AMOUNT?: string;
  TOKEN_DENOM?: string;
  PROXY_GATEWAY_ADDRESS?: string;
  ROUTER_ADDRESS?: string;
  GATEWAY_ADDRESS?: string;
  MULTISIG_ADDRESS?: string;
  MULTISIG_PROVER_ADDRESS?: string;
  VOTING_VERIFIER_ADDRESS?: string;
  REWARDS_ADDRESS?: string;
  COORDINATOR_ADDRESS?: string;
  WALLET_ADDRESS?: string;
  SALT?: string;
  EPOCH_DURATION?: string;
  RUN_AS_ACCOUNT?: string;
  CHAIN_FINALITY?: string;
  CHAIN_CONFIRMATIONS?: string;
  CHAIN_APPROX_FINALITY_WAIT_TIME?: string;
  
  // Command line args and state flags
  CONTRACT_VERSION?: string;
  IS_NEW_DEPLOYMENT?: boolean;
  VERIFIERS_REGISTERED?: boolean;
  MULTISIG_PROPOSALS_APPROVED?: boolean;
  
  [key: string]: string | boolean | undefined;
}

/**
 * Initialize base config object
 */
export const config: EnvironmentConfig = {
  NAMESPACE: '',
};

/**
 * Reset the config to its initial state
 */
export function resetConfig(): void {
  Object.keys(config).forEach(key => {
    if (key !== 'NAMESPACE') {
      delete config[key];
    }
  });
  config.NAMESPACE = '';
}