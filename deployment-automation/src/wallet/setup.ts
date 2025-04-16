/**
 * Wallet setup and management
 */

import { execSync } from 'child_process';
import { config } from '../config/environment';
import { spawnWithInput } from '../utils/exec';
import { WALLET_NAME } from '../../constants';

/**
 * Function to use an existing wallet or create a new one if needed
 */
export async function setupWallet(): Promise<void> {
  const walletName = WALLET_NAME;

  console.log(`⚡ Setting up wallet '${walletName}'...`);

  try {
    // First check if the wallet already exists
    try {
      const walletAddress = execSync(`axelard keys show "${walletName}" --keyring-backend test -a`, { stdio: 'pipe' }).toString().trim();
      if (walletAddress) {
        console.log(`✅ Using existing wallet '${walletName}': ${walletAddress}`);
        config.WALLET_ADDRESS = walletAddress;
        return;  // Exit function early since we're using an existing wallet
      }
    } catch (error) {
      // Wallet doesn't exist, we'll create it below
      console.log(`Wallet '${walletName}' not found, will create it...`);
    }

    // Only reach here if wallet doesn't exist
    if (!config.MNEMONIC) {
      throw new Error("Mnemonic is required to create a wallet");
    }
    
    // Clean up the mnemonic - remove any quotes
    const cleanMnemonic = config.MNEMONIC.replace(/^["'](.*)["']$/, '$1');
    
    // Create the wallet using spawn
    await spawnWithInput('axelard', [
      'keys',
      'add',
      walletName,
      '--keyring-backend',
      'test',
      '--recover'
    ], cleanMnemonic);
    
    // Verify wallet creation
    const walletAddress = execSync(`axelard keys show "${walletName}" --keyring-backend test -a`, { stdio: 'pipe' }).toString().trim();
    if (walletAddress) {
      console.log(`✅ Wallet successfully created! Address: ${walletAddress}`);
      config.WALLET_ADDRESS = walletAddress;
    } else {
      console.log("❌ Failed to create wallet!");
      throw new Error("Failed to create wallet");
    }
  } catch (error) {
    console.error(`Error setting up wallet: ${error}`);
    throw error;
  }
}

/**
 * Function to determine the token denomination
 */
export async function getTokenDenomination(): Promise<void> {
  console.log(`⚡ Querying wallet balance for ${config.WALLET_ADDRESS} and to determine token denomination...`);

  try {
    const balanceOutput = execSync(`axelard q bank balances "${config.WALLET_ADDRESS}" --node "${config.AXELAR_RPC_URL}"`, { stdio: 'pipe' }).toString();
    
    console.log("🔍 Wallet Balance Output:");
    console.log(balanceOutput);

    // Extract the first token denomination found
    const tokenDenomMatch = balanceOutput.match(/denom:\s+(\S+)/);
    const tokenDenom = tokenDenomMatch ? tokenDenomMatch[1] : null;

    if (!tokenDenom) {
      console.log("❌ Could not determine token denomination! Check if wallet has funds.");
      throw new Error("Could not determine token denomination");
    }

    config.TOKEN_DENOM = tokenDenom;
    console.log(`✅ Retrieved token denomination: ${tokenDenom}`);
  } catch (error) {
    console.error(`Error determining token denomination: ${error}`);
    throw error;
  }
}