const { Command } = require('commander');
const { hex } = require('./utils');
const { printInfo, printError } = require('../common');

function encodeStringToHex(inputString, options = {}) {
    const { padding = 40, uppercase = true } = options;
    
    let hexString = hex(inputString);
    
    if (uppercase) {
        hexString = hexString.toUpperCase();
    }
    
    const paddedHex = hexString + '0'.repeat(Math.max(0, padding - hexString.length));
    
    return paddedHex;
}

function processCommand(inputString, options) {
    try {
        const result = encodeStringToHex(inputString, options);
        
        if (options.format === 'detailed') {
            printInfo('Input string', inputString);
            printInfo('Hex representation', result);
            printInfo('Hex length', result.length.toString());
        } else {
            console.log(result);
        }
        
        return result;
    } catch (error) {
        printError('Failed to encode string to hex', error.message);
        process.exit(1);
    }
}

if (require.main === module) {
    const program = new Command();

    program
        .name('encode-address')
        .description('Convert strings to hex representation with padding.')
        .argument('<string>', 'The string to convert to hex')
        .option('-p, --padding <number>', 'Number of characters to pad to', '40')
        .option('--no-uppercase', 'Keep hex in lowercase')
        .option('-f, --format <format>', 'Output format: simple or detailed', 'simple')
        .action((inputString, options) => {
            const padding = parseInt(options.padding);
            if (isNaN(padding) || padding < 0) {
                printError('Invalid padding value', 'Padding must be a positive number');
                process.exit(1);
            }
            
            if (!['simple', 'detailed'].includes(options.format)) {
                printError('Invalid format', 'Format must be simple or detailed');
                process.exit(1);
            }
            
            processCommand(inputString, {
                padding,
                uppercase: options.uppercase,
                format: options.format
            });
        });

    program.parse();
}

module.exports = {
    encodeStringToHex,
    processCommand
};
