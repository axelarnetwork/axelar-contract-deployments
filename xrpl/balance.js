const { Command } = require('commander');
const { mainProcessor, getWallet, printWalletInfo } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

async function balance(_, chain, client, options) {
    const wallet = getWallet(options);
    await printWalletInfo(client, wallet, chain);
}

if (require.main === module) {
    const program = new Command();

    program.name('balance').description('Display balance of the wallet on XRPL.');

    addBaseOptions(program);

    program.action((options) => {
        mainProcessor(options, balance);
    });

    program.parse();
}
