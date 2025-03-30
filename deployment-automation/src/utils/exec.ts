/**
 * Command execution utilities
 */

import { exec, execSync, spawn } from 'child_process';
import * as util from 'util';

/**
 * Promisified version of exec
 */
export const execAsync = util.promisify(exec);

/**
 * Execute a command synchronously and capture output
 */
export function execSyncWithOutput(command: string): string {
  try {
    return execSync(command, { stdio: 'pipe' }).toString().trim();
  } catch (error) {
    console.error(`Error executing command: ${command}`);
    console.error(error);
    throw error;
  }
}

/**
 * Execute a command as a spawned process with possible user input
 */
export async function spawnWithInput(command: string, args: string[], inputText?: string): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    const process = spawn(command, args, {
      stdio: inputText ? 'pipe' : 'inherit'
    });

    if (inputText && process.stdin) {
      process.stdin.write(inputText + '\n');
      process.stdin.end();
    }

    process.on('close', (code: number) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`Command failed with exit code ${code}`));
      }
    });
    
    process.on('error', (err) => {
      reject(err);
    });
  });
}