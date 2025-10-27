'use strict';

const { Command } = require('commander');
const { addBaseOptions, addOptionsToCommands, encodeITSDestination, loadConfig, printInfo } = require('../common');

async function encodeRecipient(config, args, _) {
    const [destinationChain, destinationAddress] = args;

    const itsDestinationAddress = encodeITSDestination(config.chains, destinationChain, destinationAddress);

    printInfo('Human-readable destination address', destinationAddress);
    printInfo('Encoded ITS destination address', itsDestinationAddress);
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);

    await processor(config, args, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('its').description('Interchain Token Service common operations.');

    program
        .command('encode-recipient <destination-chain> <destination-address>')
        .description('Encode ITS recipient based on destination chain in config')
        .action((destinationChain, destinationAddress, options) => {
            mainProcessor(encodeRecipient, [destinationChain, destinationAddress], options);
        });

    addOptionsToCommands(program, addBaseOptions, { ignoreChainNames: true });

    program.parse();
}
