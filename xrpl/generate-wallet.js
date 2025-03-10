const { Command } = require('commander');
const { addWalletOptions } = require('./cli-utils');
const { generateWallet } = require('./utils');
const { printInfo } = require('../common');

function processCommand(options) {
    const wallet = generateWallet(options);
    printInfo('Generated new XRPL wallet', wallet);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('generate-wallet')
        .description('Generate a new XRPL wallet.')
        .action((options) => {
            processCommand(options);
        });

    addWalletOptions(program);

    program.parse();
}
