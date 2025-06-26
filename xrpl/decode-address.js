const { Command } = require('commander');
const { decodeAccountIDToHex } = require('./utils');
const { printInfo, printError } = require('../common');

function processCommand(address) {
    try {
        const decodedAddressHex = decodeAccountIDToHex(address);
        printInfo('Account ID raw bytes', `0x${decodedAddressHex}`);
    } catch (error) {
        printError('Failed to decode account ID', error.message);
        process.exit(1);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('decode-address').description('Decode XRPL account ID to raw bytes.').argument('<address>', 'XRPL account ID to decode');

    program.action((address) => {
        processCommand(address);
    });

    program.parse();
}
