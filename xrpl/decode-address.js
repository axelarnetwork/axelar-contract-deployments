const { Command } = require('commander');
const { decodeAccountIDToHex } = require('./utils');
const { printInfo, printError } = require('../common');

function processCommand(address) {
    try {
        // Convert the entire XRPL Base58 address to hex
        const addressHex = Buffer.from(address, 'utf8').toString('hex');
        
        printInfo('XRPL Address (Base58)', address);
        printInfo('XRPL Address (Hex)', `0x${addressHex}`);
        
        // If you also want the raw account bytes (for reference)
        const decodedAddressHex = decodeAccountIDToHex(address);
        printInfo('Account ID raw bytes', `0x${decodedAddressHex}`);
        
        return addressHex; // Return the hex-encoded full address
    } catch (error) {
        printError('Failed to decode account ID', error.message);
        process.exit(1);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('decode-address').description('Decode XRPL account ID to raw bytes.').argument('<address>', 'XRPL account ID to decode');

    program.action((address) => {
        const result = processCommand(address);
        // You can use the result if needed
    });

    program.parse();
}
