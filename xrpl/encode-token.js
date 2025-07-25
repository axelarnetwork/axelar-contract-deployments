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

function processCommand(tokenSymbol) {
    const result = tokenSymbolToCurrencyCode(inputString);
    printInfo('Input string', inputString);
    printInfo('Hex representation', result);
    return result;
}

if (require.main === module) {
    const program = new Command();

    program
        .name('token')
        .description('Convert token symbol to XRPL Currency Code.')
        .argument('<token-symbol>', 'The token symbol to convert to XRPL currency code')
        .action((tokenSymbol) => {
            processCommand(tokenSymbol);
        });

    program.parse();
}

module.exports = {
    tokenSymbolToCurrencyCode,
    processCommand,
};
