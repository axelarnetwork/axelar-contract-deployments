const xrpl = require('xrpl');
const { Command } = require('commander');
const { printInfo } = require('../common');

const KEY_TYPE = xrpl.ECDSA.secp256k1;

async function processCommand(_) {
    const wallet = xrpl.Wallet.generate(KEY_TYPE);
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
