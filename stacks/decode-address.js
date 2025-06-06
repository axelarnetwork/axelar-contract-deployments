'use strict';

const { printInfo } = require('../common/utils');
const { Command } = require('commander');
const { cvToHex, principalCV } = require('@stacks/transactions');

function processCommand(stacksAddress) {
    printInfo('Stacks address is:', stacksAddress);
    printInfo('Decoded as hex:', cvToHex(principalCV((stacksAddress))));
}

if (require.main === module) {
    const program = new Command();

    program
        .name('decode-address')
        .description('Decode the Stacks address to hex.')
        .argument('<stacksAddress>', 'Stacks address to decode')
        .action((stacksAddress) => {
            processCommand(stacksAddress);
        });

    program.parse();
}
