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
  try {
    const privateKey = getRequiredEnvVar('TARGET_CHAIN_PRIVATE_KEY');
    
    if (!/^0x[0-9a-fA-F]+$/.test(privateKey)) {
      throw new Error("Invalid private key format. Make sure it starts with '0x' and contains only hexadecimal characters (0-9, a-f).");
    }
    
    config.TARGET_CHAIN_PRIVATE_KEY = privateKey;
    displayMessage(MessageType.SUCCESS, "Valid private key loaded from environment");
  } catch (error) {
    displayMessage(MessageType.ERROR, `Private key validation failed: ${error}`);
    displayMessage(MessageType.INFO, "Please add a valid TARGET_CHAIN_PRIVATE_KEY to your .env file");
    createEnvTemplate();
    throw error;
  }
}

/**
 * Function to validate and load RPC URL from environment
 */
export function validateRpcUrl(): void {
  try {
    const rpcUrl = getRequiredEnvVar('RPC_URL');
    
    if (!/^https?:\/\//.test(rpcUrl)) {
      throw new Error("Invalid RPC URL format. It must start with 'http://' or 'https://'.");
    }
    
    config.RPC_URL = rpcUrl;
    displayMessage(MessageType.SUCCESS, `RPC URL loaded: ${rpcUrl}`);
  } catch (error) {
    displayMessage(MessageType.ERROR, `RPC URL validation failed: ${error}`);
    displayMessage(MessageType.INFO, "Please add a valid RPC_URL to your .env file");
    createEnvTemplate();
    throw error;
  }
}

/**
 * Function to validate and load Axelar RPC Node URL from environment
 */
export function validateAxelarRpcUrl(): void {
  try {
    const axelarRpcUrl = getRequiredEnvVar('AXELAR_RPC_URL');
    
    if (!/^https?:\/\//.test(axelarRpcUrl)) {
      throw new Error("Invalid Axelar RPC Node URL format. It must start with 'http://' or 'https://'.");
    }
    
    config.AXELAR_RPC_URL = axelarRpcUrl;
    displayMessage(MessageType.SUCCESS, `Axelar RPC URL loaded: ${axelarRpcUrl}`);
  } catch (error) {
    displayMessage(MessageType.ERROR, `Axelar RPC URL validation failed: ${error}`);
    displayMessage(MessageType.INFO, "Please add a valid AXELAR_RPC_URL to your .env file");
    createEnvTemplate();
    throw error;
  }
}

/**
 * Function to validate and load mnemonic from environment
 */
export function validateMnemonic(): void {
  try {
    const mnemonic = getRequiredEnvVar('MNEMONIC');
    
    // Basic mnemonic validation - typically 12, 15, 18, 21, or 24 words
    const wordCount = mnemonic.trim().split(/\s+/).length;
    const validWordCounts = [12, 15, 18, 21, 24];
    
    if (!validWordCounts.includes(wordCount)) {
      throw new Error(`Invalid mnemonic: expected 12, 15, 18, 21, or 24 words but got ${wordCount}`);
    }
    
    config.MNEMONIC = mnemonic;
    displayMessage(MessageType.SUCCESS, "Valid mnemonic loaded from environment");
  } catch (error) {
    displayMessage(MessageType.ERROR, `Mnemonic validation failed: ${error}`);
    displayMessage(MessageType.INFO, "Please add a valid MNEMONIC to your .env file");
    createEnvTemplate();
    throw error;
  }
}

/**
 * Function to validate and load basic chain information from environment
 */
export function validateChainInfo(): void {
  try {
    // Chain name is required
    const chainName = getRequiredEnvVar('CHAIN_NAME');
    config.CHAIN_NAME = chainName;
    displayMessage(MessageType.SUCCESS, `Chain name loaded: ${chainName}`);
    
    // Chain ID is required
    const chainId = getRequiredEnvVar('CHAIN_ID');
    config.CHAIN_ID = chainId;
    displayMessage(MessageType.SUCCESS, `Chain ID loaded: ${chainId}`);
    
    // Token symbol is required
    const tokenSymbol = getRequiredEnvVar('TOKEN_SYMBOL');
    config.TOKEN_SYMBOL = tokenSymbol;
    displayMessage(MessageType.SUCCESS, `Token symbol loaded: ${tokenSymbol}`);
    
    // Gas limit is required
    const gasLimit = getRequiredEnvVar('GAS_LIMIT');
    config.GAS_LIMIT = gasLimit;
    displayMessage(MessageType.SUCCESS, `Gas limit loaded: ${gasLimit}`);
  } catch (error) {
    displayMessage(MessageType.ERROR, `Chain info validation failed: ${error}`);
    displayMessage(MessageType.INFO, "Please add the required chain information to your .env file");
    createEnvTemplate();
    throw error;
  }
}