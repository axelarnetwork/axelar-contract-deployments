const { Command } = require('commander');
const { mainProcessor, getWallet } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

async function processCommand(_, chain, options) {
    await getWallet(chain, options);
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('balance').description('Display balance of the wallet on XRPL.');

    addBaseOptions(program);

    program.action((options) => {
        main(options);
    });

    program.parse();
}
