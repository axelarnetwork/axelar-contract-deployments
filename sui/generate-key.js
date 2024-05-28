'use strict';

const { addBaseOptions } = require('./cli-utils');
const { generateKeypair } = require('./sign-utils');
const { Command, Option } = require('commander');
const { saveConfig, loadConfig, printInfo } = require('../evm/utils');

async function processCommand(config, chain, options) {
    const [keypair, _] = await generateKeypair(options);

    printInfo('Keypair generated');
    printInfo('Public key', keypair.getPublicKey());
    printInfo('Address', keypair.toSuiAddress())
    printInfo('Private key', keypair.export());
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('generate-key').description('Generate keypair.');

    addBaseOptions(program, { ignorePrivateKey: true });

    program.addOption(
        new Option('--signatureScheme <signatureScheme>', 'sig scheme').choices(['ed25519', 'secp256k1', 'secp256r1']).default('ed25519'),
    );

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
