/**
 * Contract download functions
 */

import * as fs from 'fs';
import * as path from 'path';
import { BASE_URL, WASM_DIR } from '../../constants';
import { ContractFile, ContractType, contractTypeToFileName } from './types';

// This would need proper implementation with axios
import axios from 'axios';

/**
 * Get the latest version of a contract
 */
export async function getLatestVersion(contractDir: string): Promise<string | null> {
  try {
    const baseUrl = `${BASE_URL}/${contractDir}/`;
    
    // This would need to be implemented with a HTTP request to fetch versions
    // For now, we'll just log that this needs to be implemented with a proper HTTP client
    console.log(`⚠️ getLatestVersion needs to be implemented with HTTP requests to ${baseUrl}`);
    console.log(`⚠️ For now, please provide a specific version number when prompted.`);
    return null;
  } catch (error) {
    console.error(`Error getting latest version: ${error}`);
    return null;
  }
}

/**
 * Function to download contract files from remote source
 */
export async function downloadContractFiles(userVersion: string): Promise<Map<string, ContractFile>> {
  console.log("⚡ Downloading contract files...");
  
  // Ensure the directory for downloads exists
  fs.mkdirSync(WASM_DIR, { recursive: true });

  // List of contract directories to check
  const contractDirectories = [
    ContractType.Gateway,
    ContractType.MultisigProver,
    ContractType.VotingVerifier
  ];

  // Map to store contract file information
  const contractFiles = new Map<string, ContractFile>();

  // Loop through each contract directory and get the files
  for (const dir of contractDirectories) {
    const fileName = contractTypeToFileName(dir);  // Convert hyphens to underscores
    const contractKey = fileName;  // Use as key for the map
    
    const wasmFilePath = path.join(WASM_DIR, `${fileName}.wasm`);
    const checksumFilePath = path.join(WASM_DIR, `${fileName}_checksums.txt`);
    
    if (!userVersion) {
      const latestVersion = await getLatestVersion(dir);
      if (!latestVersion) {
        console.error("❌ No version specified and getLatestVersion is not fully implemented.");
        throw new Error("Version required but not provided");
      }
    }

    const version = userVersion || "latest";
    const fileUrl = `${BASE_URL}/${dir}/${version}/${fileName}.wasm`;
    const checksumUrl = `${BASE_URL}/${dir}/${version}/checksums.txt`;

    console.log(`⬇️ Downloading ${fileUrl}...`);
    
    try {
      // Download the WASM file
      const wasmResponse = await axios({
        method: 'GET',
        url: fileUrl,
        responseType: 'arraybuffer'
      });
      
      // Write the WASM file to disk
      fs.writeFileSync(wasmFilePath, Buffer.from(wasmResponse.data));
      
      console.log(`✅ Downloaded ${fileName}.wasm successfully!`);
      
      // Download checksum file
      console.log(`⬇️ Downloading ${checksumUrl}...`);
      const checksumResponse = await axios({
        method: 'GET',
        url: checksumUrl,
        responseType: 'text'
      });
      
      // Write the checksum file to disk
      fs.writeFileSync(checksumFilePath, checksumResponse.data);
      
      console.log(`✅ Downloaded checksums.txt successfully!`);
      
      // Store the contract file information
      contractFiles.set(contractKey, {
        name: dir,
        fileName: fileName,
        filePath: wasmFilePath,
        checksumPath: checksumFilePath
      });
    } catch (error) {
      console.error(`❌ Error downloading files: ${error}`);
      throw new Error(`Failed to download contract files: ${error}`);
    }
  }

  return contractFiles;
}