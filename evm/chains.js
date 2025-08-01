'use strict';

const { Command } = require('commander');
const { mainProcessor, printInfo } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

async function processCommand(_axelar, chain, _chains, _options) {
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
