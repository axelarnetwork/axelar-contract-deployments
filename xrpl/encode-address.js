const { Command } = require('commander');
const { hex } = require('./utils');
const { printInfo, printError } = require('../common');

function encodeStringToHex(inputString) {
    if (inputString.length <= 3) {
        throw new Error('String must be longer than 3 characters');
    }

    const hexString = hex(inputString).toUpperCase();
    const paddedHex = hexString + '0'.repeat(Math.max(0, 40 - hexString.length));

    return paddedHex;
}

function processCommand(inputString) {
    try {
        const result = encodeStringToHex(inputString);
        printInfo('Input string', inputString);
        printInfo('Hex representation', result);
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
        .description('Convert strings to 40-character padded hex representation.')
        .argument('<string>', 'The string to convert to hex (must be longer than 3 characters)')
        .action((inputString) => {
            processCommand(inputString);
        });

    program.parse();
}

module.exports = {
    encodeStringToHex,
    processCommand,
};
