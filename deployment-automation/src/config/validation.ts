/**
 * Input validation functions
 */

import { config } from './environment';
import { question, displayMessage, MessageType } from '../utils/cli-utils';
import { getEnvVar, getRequiredEnvVar, createEnvTemplate } from '../utils/env';

/**
 * Function to validate and load private key from environment
 */
export function validatePrivateKey(): void {
  const privateKey = config.PRIVATE_KEY;
  
  if (!privateKey) {
    throw new Error('Private key is required');
  }

  // First check for general format (hexadecimal characters)
  if (!/^(0x)?[0-9a-fA-F]+$/.test(privateKey)) {
    throw new Error('Invalid private key format');
  }

  // Then check for 0x prefix
  if (!privateKey.startsWith('0x')) {
    throw new Error('Private key must start with 0x');
  }

  // Finally check for specific length
  if (!/^0x[0-9a-fA-F]{64}$/.test(privateKey)) {
    throw new Error('Private key must be 64 characters long (excluding 0x prefix)');
  }

  displayMessage(MessageType.SUCCESS, "Valid private key loaded from environment");
}

/**
 * Function to validate and load RPC URL from environment
 */
export function validateRpcUrl(): void {
  const rpcUrl = config.RPC_URL;
  
  if (!rpcUrl) {
    throw new Error('RPC URL is required');
  }

  if (!/^https?:\/\//.test(rpcUrl)) {
    throw new Error('Invalid RPC URL format');
  }

  displayMessage(MessageType.SUCCESS, `RPC URL loaded: ${rpcUrl}`);
}

/**
 * Function to validate and load Axelar RPC Node URL from environment
 */
export function validateAxelarRpcUrl(): void {
  const axelarRpcUrl = config.AXELAR_RPC_URL;
  
  if (!axelarRpcUrl) {
    throw new Error('Axelar RPC URL is required');
  }

  if (!/^https?:\/\//.test(axelarRpcUrl)) {
    throw new Error('Invalid Axelar RPC URL format');
  }

  displayMessage(MessageType.SUCCESS, `Axelar RPC URL loaded: ${axelarRpcUrl}`);
}

/**
 * Function to validate and load mnemonic from environment
 */
export function validateMnemonic(): void {
  const mnemonic = config.MNEMONIC;
  
  if (!mnemonic) {
    throw new Error('Mnemonic is required');
  }

  // Basic mnemonic validation - typically 12 or 24 words
  const wordCount = mnemonic.trim().split(/\s+/).length;
  if (wordCount !== 12 && wordCount !== 24) {
    throw new Error('Mnemonic must contain 12 or 24 words');
  }

  displayMessage(MessageType.SUCCESS, "Valid mnemonic loaded from environment");
}

/**
 * Function to validate and load basic chain information from environment
 */
export function validateChainInfo(): void {
  // Chain name validation
  if (!config.CHAIN_NAME) {
    throw new Error('Chain name is required');
  }

  // Chain ID validation
  if (!config.CHAIN_ID) {
    throw new Error('Chain ID is required');
  }

  if (isNaN(Number(config.CHAIN_ID))) {
    throw new Error('Chain ID must be a number');
  }

  // Token symbol validation
  if (!config.TOKEN_SYMBOL) {
    throw new Error('Token symbol is required');
  }

  // Gas limit validation
  if (!config.GAS_LIMIT) {
    throw new Error('Gas limit is required');
  }

  if (isNaN(Number(config.GAS_LIMIT))) {
    throw new Error('Gas limit must be a number');
  }

  displayMessage(MessageType.SUCCESS, "Chain info validation completed successfully");
}