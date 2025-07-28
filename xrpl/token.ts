import { Command } from 'commander';

import { printInfo } from '../common';
import { hex } from './utils';

const STANDARD_CURRENCY_CHARS = /^[A-Za-z0-9?!@#$%^&*<>(){}[\]|]+$/;

function tokenSymbolToCurrencyCode(tokenSymbol: string): string {
    if (tokenSymbol.length <= 3) {
        if (!STANDARD_CURRENCY_CHARS.test(tokenSymbol)) {
            throw new Error(
                `Token symbol "${tokenSymbol}" contains invalid characters. Standard currency codes (≤3 chars) must only contain: letters, digits, and symbols: ?!@#$%^&*<>(){}[]|`,
            );
        }
        return tokenSymbol;
    }

    const tokenSymbolHex = hex(tokenSymbol).toUpperCase();

    if (tokenSymbolHex.length > 40) {
        throw new Error(
            `Token symbol "${tokenSymbol}" too long: hex representation (${tokenSymbolHex.length} chars) exceeds xrpl 40-character limit`,
        );
    }
    const currencyCode = tokenSymbolHex + '0'.repeat(40 - tokenSymbolHex.length);

    return currencyCode;
}

function currencyCodeToTokenSymbol(currencyCode: string): string {
    if (currencyCode.length <= 3) {
        if (!STANDARD_CURRENCY_CHARS.test(currencyCode)) {
            throw new Error(
                `Currency code "${currencyCode}" contains invalid characters. Standard currency codes (≤3 chars) must only contain: letters, digits, and symbols: ?!@#$%^&*<>(){}[]|`,
            );
        }
        return currencyCode;
    }

    if (currencyCode.length !== 40) {
        throw new Error(`Invalid currency code: ${currencyCode} must be exactly 40 characters (160-bit hex)`);
    }

    if (!/^[0-9A-Fa-f]+$/.test(currencyCode)) {
        throw new Error(`Invalid currency code: ${currencyCode} is not a valid hex string`);
    }

    const trimmedcurrencyCode = currencyCode.replace(/0+$/, '');

    if (trimmedcurrencyCode.length % 2 !== 0 || trimmedcurrencyCode.length < 8) {
        throw new Error(`Invalid currency code: ${currencyCode} has invalid hex length after trimming`);
    }

    const buffer = Buffer.from(trimmedcurrencyCode, 'hex');
    const tokenSymbol = buffer.toString('ascii');

    if (tokenSymbol.length <= 3) {
        throw new Error(
            `Invalid currency code: ${currencyCode} decodes to "${tokenSymbol}" which is ≤3 characters and would be ambiguous with standard currency codes`,
        );
    }

    return tokenSymbol;
}

function processTokenSymbolToCurrencyCode(tokenSymbol: string): string {
    const currencyCode = tokenSymbolToCurrencyCode(tokenSymbol);
    printInfo('Token Symbol', tokenSymbol);
    printInfo('XRPL Currency Code', currencyCode);
    return currencyCode;
}

function processCurrencyCodeToTokenSymbol(currencyCode: string): string {
    const tokenSymbol = currencyCodeToTokenSymbol(currencyCode);
    printInfo('XRPL Currency Code', currencyCode);
    printInfo('Token Symbol', tokenSymbol);
    return tokenSymbol;
}

/**
 * XRPL Token Symbol <-> Currency Code Converter
 * Implements XRPL currency code standards as documented at:
 * https://xrpl.org/docs/references/protocol/data-types/currency-formats#currency-codes
 */
if (require.main === module) {
    const program = new Command();

    program.name('token').description('Convert between token symbols and XRPL Currency Codes.');

    program
        .command('token-symbol-to-currency-code')
        .description('Convert token symbol to XRPL Currency Code')
        .argument('<token-symbol>', 'The token symbol to convert to XRPL currency code')
        .action((tokenSymbol: string) => {
            processTokenSymbolToCurrencyCode(tokenSymbol);
        });

    program
        .command('currency-code-to-token-symbol')
        .description('Convert XRPL Currency Code to token symbol')
        .argument('<currency-code>', 'The XRPL currency code to convert to token symbol')
        .action((currencyCode: string) => {
            processCurrencyCodeToTokenSymbol(currencyCode);
        });

    program.parse();
}

export { tokenSymbolToCurrencyCode, currencyCodeToTokenSymbol, processTokenSymbolToCurrencyCode, processCurrencyCodeToTokenSymbol };
