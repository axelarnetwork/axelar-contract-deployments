/**
 * Contract-related interfaces and types
 */

/**
 * Contract file information interface
 */
export interface ContractFile {
    name: string;          // Contract name (e.g., "gateway")
    fileName: string;      // File name with underscores (e.g., "gateway" or "voting_verifier")
    filePath: string;      // Full path to the .wasm file
    checksumPath: string;  // Full path to the checksums file
  }
  
  /**
   * Types of contracts that can be deployed
   */
  export enum ContractType {
    Gateway = 'gateway',
    MultisigProver = 'multisig-prover',
    VotingVerifier = 'voting-verifier'
  }
  
  /**
   * Convert contract type to file name (replacing hyphens with underscores)
   */
  export function contractTypeToFileName(type: ContractType): string {
    return type.replace(/-/g, '_');
  }