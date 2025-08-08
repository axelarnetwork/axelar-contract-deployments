const { Command } = require('commander');
const { mainProcessor, getMultisigProof, broadcastTxBlob } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');
const { printError } = require('../common');

async function broadcast(config, _wallet, client, chain, options, args) {
    const { status } = await getMultisigProof(config.axelar, chain.axelarId, args.multisigSessionId, 'XrplMultisigProver');

    if (!status.completed) {
        printError(`Multisig session ${args.multisigSessionId} not completed`);
        process.exit(1);
    }

    const txBlob = status.completed.execute_data;
    await broadcastTxBlob(client, txBlob, options);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('submit-proof')
        .description('Submit proof to XRPL for the provided amplifier multisig session ID.')
        .argument('<multisigSessionId>', 'multisig session ID to submit proof for')
        .action((multisigSessionId, options) => {
            mainProcessor(broadcast, options, { multisigSessionId });
        });

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.parse();
}
