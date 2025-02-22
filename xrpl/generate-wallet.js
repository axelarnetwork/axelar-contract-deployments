const { Command } = require('commander');
const { generateWallet } = require('./utils');
const { printInfo } = require('../common');

function processCommand(_) {
    const wallet = generateWallet();
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

    program.parse();
}
