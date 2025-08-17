import * as fs from 'fs';
import * as path from 'path';

import { printInfo } from '../../common/utils';
import { isValidCosmosAddress } from '../utils';
import { DEFAULTS } from './constants';
import type { CoordinatorOptions } from './types';

function isValidHexString(hexString: string): boolean {
    // Check if the string is not empty and has even length
    if (!hexString || hexString.length % 2 !== 0) {
        return false;
    }

    // Check if all characters are valid hex digits (0-9, a-f, A-F)
    const hexPattern = /^[0-9a-fA-F]+$/;
    return hexPattern.test(hexString);
}

export class OptionProcessor {
    private static envCache: Map<string, string> | null = null;

    private static loadEnvFile(): Map<string, string> {
        if (this.envCache) {
            return this.envCache;
        }

        this.envCache = new Map();
        const envPath = path.join(process.cwd(), '.env');

        if (fs.existsSync(envPath)) {
            try {
                const envContent = fs.readFileSync(envPath, 'utf8');
                const envLines = envContent.split('\n');

                for (const line of envLines) {
                    const trimmedLine = line.trim();
                    if (trimmedLine && !trimmedLine.startsWith('#')) {
                        const [key, ...valueParts] = trimmedLine.split('=');
                        if (key && valueParts.length > 0) {
                            const value = valueParts.join('=');
                            this.envCache.set(key, value);
                        }
                    }
                }
            } catch (error) {
                printInfo('Failed to load .env file:', error);
            }
        }

        return this.envCache;
    }

    private static getEnvValue(key: string): string | undefined {
        return this.loadEnvFile().get(key);
    }

    private static parseThreshold(value: string | [string, string] | undefined, defaultThreshold: [string, string]): [string, string] {
        if (!value) return defaultThreshold;
        if (typeof value === 'string') {
            const parts = value.split(',').map((s) => s.trim());
            return parts.length === 2 ? (parts as [string, string]) : defaultThreshold;
        }
        return value;
    }

    private static parseNumber(value: number | string | undefined, defaultValue: number): number {
        return value ? parseInt(value.toString(), 10) : defaultValue;
    }

    public static processOptions(options: CoordinatorOptions): CoordinatorOptions {
        const processedOptions = { ...options };

        // Check for mnemonic in environment variable or .env file if not provided via command line
        if (!processedOptions.mnemonic) {
            const envMnemonic = process.env.MNEMONIC || this.getEnvValue('MNEMONIC');
            if (envMnemonic) {
                processedOptions.mnemonic = envMnemonic;
            }
        }

        // Check for environment in environment variable or .env file if not provided via command line
        if (!processedOptions.env) {
            const envEnvironment = process.env.ENVIRONMENT || this.getEnvValue('ENVIRONMENT');
            if (envEnvironment) {
                processedOptions.env = envEnvironment;
            }
        }

        // Check for chain name in environment variable or .env file if not provided via command line
        if (!processedOptions.chain && !processedOptions.chainName) {
            const envChainName = process.env.CHAIN_NAME || this.getEnvValue('CHAIN_NAME');
            if (envChainName) {
                processedOptions.chain = envChainName;
            }
        }

        // Process thresholds
        processedOptions.votingThreshold = this.parseThreshold(options.votingThreshold, DEFAULTS.votingThreshold);
        processedOptions.signingThreshold = this.parseThreshold(options.signingThreshold, DEFAULTS.signingThreshold);

        // Process numeric values
        processedOptions.confirmationHeight = this.parseNumber(options.confirmationHeight, DEFAULTS.confirmationHeight);
        processedOptions.verifierSetDiffThreshold = this.parseNumber(options.verifierSetDiffThreshold, DEFAULTS.verifierSetDiffThreshold);

        // Set defaults for string values
        const stringDefaults = {
            serviceName: DEFAULTS.serviceName,
            blockExpiry: DEFAULTS.blockExpiry,
            msgIdFormat: DEFAULTS.msgIdFormat,
            addressFormat: DEFAULTS.addressFormat,
            encoder: DEFAULTS.encoder,
            keyType: DEFAULTS.keyType,
        };

        Object.entries(stringDefaults).forEach(([key, defaultValue]) => {
            const typedKey = key as keyof Pick<
                CoordinatorOptions,
                'serviceName' | 'blockExpiry' | 'msgIdFormat' | 'addressFormat' | 'encoder' | 'keyType'
            >;
            if (!processedOptions[typedKey]) {
                processedOptions[typedKey] = defaultValue;
            }
        });

        this.validateOptions(processedOptions);
        return processedOptions;
    }

    private static validateThreshold(threshold: [string, string], name: string): void {
        const [numerator, denominator] = threshold;

        if (isNaN(parseInt(numerator, 10)) || isNaN(parseInt(denominator, 10))) {
            throw new Error(`${name} values must be valid numbers`);
        }

        if (parseInt(numerator, 10) > parseInt(denominator, 10)) {
            throw new Error(`${name} numerator cannot be greater than denominator`);
        }

        if (parseInt(numerator, 10) <= 0 || parseInt(denominator, 10) <= 0) {
            throw new Error(`${name} values must be positive numbers`);
        }
    }

    private static validateOptions(options: CoordinatorOptions): void {
        if (options.votingThreshold) {
            if (typeof options.votingThreshold === 'string') {
                const parts = options.votingThreshold.split(',').map((s) => s.trim());
                if (parts.length === 2) {
                    this.validateThreshold([parts[0], parts[1]], 'Voting threshold');
                } else {
                    throw new Error('Voting threshold must be in format "numerator,denominator"');
                }
            } else {
                this.validateThreshold(options.votingThreshold, 'Voting threshold');
            }
        }
        if (options.signingThreshold) {
            if (typeof options.signingThreshold === 'string') {
                const parts = options.signingThreshold.split(',').map((s) => s.trim());
                if (parts.length === 2) {
                    this.validateThreshold([parts[0], parts[1]], 'Signing threshold');
                } else {
                    throw new Error('Signing threshold must be in format "numerator,denominator"');
                }
            } else {
                this.validateThreshold(options.signingThreshold, 'Signing threshold');
            }
        }

        this.validateAddressesForCommand(options);
        this.validateDomainSeparator(options.domainSeparator);
        this.validateSalt(options.salt);
    }

    private static validateAddressesForCommand(options: CoordinatorOptions): void {
        if (options.governanceAddress && !isValidCosmosAddress(options.governanceAddress)) {
            throw new Error('Invalid governance address format');
        }

        if (options.rewardsAddress && !isValidCosmosAddress(options.rewardsAddress)) {
            throw new Error('Invalid rewards address format');
        }
    }

    private static validateDomainSeparator(domainSeparator?: string): void {
        if (!domainSeparator) return;

        if (!domainSeparator.startsWith('0x')) {
            throw new Error('Domain separator must start with 0x');
        }

        if (!/^0x[a-fA-F0-9]{64}$/.test(domainSeparator)) {
            throw new Error('Domain separator must be a valid 32-byte hex string');
        }
    }

    private static validateSalt(salt?: string): void {
        if (!salt) return;

        if (!isValidHexString(salt)) {
            throw new Error(
                `Invalid salt format. Salt must be a valid hex string (even number of hex digits: 0-9, a-f, A-F). Provided: "${salt}"`,
            );
        }
    }
}
