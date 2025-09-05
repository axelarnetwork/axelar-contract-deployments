const { Command } = require('commander');
const { mainProcessor, broadcastTxBlob } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');

async function broadcast(_config, _wallet, client, _chain, options, args) {
    const { txBlob } = args;
    await broadcastTxBlob(client, txBlob, options);
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
