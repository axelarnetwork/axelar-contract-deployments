'use strict';

const { Command } = require('commander');
const { mainProcessor, printInfo } = require('./utils');
const { addBaseOptions } = require('../common');

async function processCommand(_, chain, options) {
    printInfo('Axelar Chain Name', chain.axelarId);
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('chains').description('Display chain names and axelar chain ids.');

    addBaseOptions(program);

    program.action((options) => {
        main(options);
    });

    program.parse();
}
