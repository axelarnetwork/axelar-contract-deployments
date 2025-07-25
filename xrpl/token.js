const { Command } = require('commander');
const { hex } = require('./utils');
const { printInfo, printError } = require('../common');

function tokenSymbolToCurrencyCode(tokenSymbol) {
    if (tokenSymbol.length <= 3) {
        return tokenSymbol;
    }

    const hexString = hex(tokenSymbol).toUpperCase();
    const paddedHex = hexString + '0'.repeat(Math.max(0, 40 - hexString.length));

    return paddedHex;
}

function currencyCodeToTokenSymbol(currencyCode) {
    if (currencyCode.length <= 3) {
        return currencyCode;
    }

    const trimmedHex = currencyCode.replace(/0+$/, '');
    
    if (!/^[0-9A-Fa-f]+$/.test(trimmedHex)) {
        printError(`Invalid currency code: ${currencyCode} is not a valid hex string`);
        process.exit(1);
    }
    
    const buffer = Buffer.from(trimmedHex, 'hex');
    return buffer.toString('utf8');
}

function processCommand(tokenSymbol) {
    const currencyCode = tokenSymbolToCurrencyCode(tokenSymbol);
    printInfo('Token Symbol', tokenSymbol);
    printInfo('XRPL Currency Code', currencyCode);
    return currencyCode;
}

function processDecodeCommand(currencyCode) {
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

    program
        .name('token')
        .description('Convert between token symbols and XRPL Currency Codes.');

    program
        .command('encode')
        .description('Convert token symbol to XRPL Currency Code')
        .argument('<token-symbol>', 'The token symbol to convert to XRPL currency code')
        .action((tokenSymbol) => {
            processCommand(tokenSymbol);
        });

    program
        .command('decode')
        .description('Convert XRPL Currency Code to token symbol')
        .argument('<currency-code>', 'The XRPL currency code to convert to token symbol')
        .action((currencyCode) => {
            processDecodeCommand(currencyCode);
        });

    program.parse();
}

module.exports = {
    tokenSymbolToCurrencyCode,
    currencyCodeToTokenSymbol,
    processCommand,
    processDecodeCommand,
};
