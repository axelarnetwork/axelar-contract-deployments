/**
 * File system operations
 */

import * as fs from 'fs';
import * as path from 'path';
import { displayMessage, MessageType } from './cli';

/**
 * Ensure a directory exists, create it if it doesn't
 */
export function ensureDirectoryExists(dirPath: string): void {
  if (!fs.existsSync(dirPath)) {
    try {
      fs.mkdirSync(dirPath, { recursive: true });
      displayMessage(MessageType.SUCCESS, `Created directory: ${dirPath}`);
    } catch (error) {
      displayMessage(MessageType.ERROR, `Failed to create directory ${dirPath}: ${error}`);
      throw error;
    }
  }
}

/**
 * Save data to a JSON file
 */
export function saveJsonToFile(filePath: string, data: any): void {
  try {
    ensureDirectoryExists(path.dirname(filePath));
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2));
    displayMessage(MessageType.SUCCESS, `Saved data to ${filePath}`);
  } catch (error) {
    displayMessage(MessageType.ERROR, `Failed to save data to ${filePath}: ${error}`);
    throw error;
  }
}

/**
 * Load data from a JSON file
 */
export function loadJsonFromFile(filePath: string): any {
  try {
    if (!fs.existsSync(filePath)) {
      displayMessage(MessageType.ERROR, `File not found: ${filePath}`);
      throw new Error(`File not found: ${filePath}`);
    }
    
    const fileContent = fs.readFileSync(filePath, 'utf-8');
    return JSON.parse(fileContent);
  } catch (error) {
    displayMessage(MessageType.ERROR, `Failed to load data from ${filePath}: ${error}`);
    throw error;
  }
}

/**
 * Check if a file exists
 */
export function fileExists(filePath: string): boolean {
  return fs.existsSync(filePath);
}