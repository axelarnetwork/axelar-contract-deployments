import { isValidCosmosAddress } from '../utils';
import type { CoordinatorOptions } from './types';

export class OptionProcessor {
    private static parseThreshold(value: string | [string, string] | undefined): [string, string] {
        if (typeof value === 'string') {
            const parts = value.split(',').map((s) => s.trim());
            if (parts.length === 2) {
                return parts as [string, string];
            } else {
                throw new Error('Threshold must be in format "numerator,denominator"');
            }
        }
        return value as [string, string];
    }

    private static parseNumber(value: number | string | undefined): number {
        if (value) {
            return parseInt(value.toString(), 10);
        } else {
            throw new Error('Value must be a number');
        }
    }

    public static processOptions(options: CoordinatorOptions): CoordinatorOptions {
        const processedOptions = { ...options };

        // Only process fields that exist in the options
        if ('votingThreshold' in processedOptions) {
            processedOptions.votingThreshold = this.parseThreshold(options.votingThreshold);
        }
        if ('signingThreshold' in processedOptions) {
            processedOptions.signingThreshold = this.parseThreshold(options.signingThreshold);
        }
        if ('confirmationHeight' in processedOptions) {
            processedOptions.confirmationHeight = this.parseNumber(options.confirmationHeight);
        }
        if ('verifierSetDiffThreshold' in processedOptions) {
            processedOptions.verifierSetDiffThreshold = this.parseNumber(options.verifierSetDiffThreshold);
        }

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
}
