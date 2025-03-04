const { decode } = require('ripple-binary-codec');
const { Command } = require('commander');
const { mainProcessor, printInfo, prompt } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');

async function broadcast(_config, _wallet, client, _chain, options, args) {
    const { txBlob } = args;
    const tx = decode(txBlob);
    printInfo('Preparing to broadcast transaction', tx);

    if (prompt(`Submit ${tx.TransactionType} transaction?`, options.yes)) {
        process.exit(0);
    }

    await client.submitTx(txBlob);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('broadcast')
        .description('Broadcast encoded signed transaction to XRPL.')
        .argument('<txBlob>', 'signed transaction blob to broadcast')
        .action((txBlob, options) => {
            mainProcessor(broadcast, options, { txBlob });
        });

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.parse();
}
