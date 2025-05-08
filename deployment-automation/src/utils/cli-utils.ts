/**
 * CLI interaction utilities
 */

import * as readline from 'readline';

// Create an interface for readline
const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

/**
 * Promisified version of readline question
 */
export const question = (query: string): Promise<string> => {
  return new Promise((resolve) => {
    rl.question(query, resolve);
  });
};

/**
 * Close the readline interface
 */
export function closeReadline(): void {
  rl.close();
}

/**
 * Display a message with a specific color
 */
export enum MessageType {
  INFO,
  SUCCESS,
  WARNING,
  ERROR
}

/**
 * Display a formatted message
 */
export function displayMessage(type: MessageType, message: string): void {
  switch(type) {
    case MessageType.INFO:
      console.info(`ℹ️ ${message}`);
      break;
    case MessageType.SUCCESS:
      console.log(`✅ ${message}`);
      break;
    case MessageType.WARNING:
      console.warn(`⚠️ ${message}`);
      break;
    case MessageType.ERROR:
      console.error(`❌ ${message}`);
      break;
    default:
      console.log(message);
  }
}

/**
 * Display a header
 */
export function displayHeader(text: string): void {
  const line = '='.repeat(text.length + 4);
  console.log(line);
  console.log(`= ${text} =`);
  console.log(line);
}