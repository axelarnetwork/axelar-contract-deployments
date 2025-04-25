'use strict';

const { Command, Option } = require('commander');
const { addBaseOptions, generateKeypair } = require('./utils');
const { loadConfig, printInfo, getChainConfig } = require('../common/utils');
const { Horizon } = require('@stellar/stellar-sdk');

async function processCommand(chain, options) {
    const keypair = await generateKeypair(options);
    const horizonServer = new Horizon.Server(chain.horizonRpc);

    // Initializes the account on-chain; without this call, the account does not yet exist.
    // Friendbot funds and activates the account (only available on testnets)
    await horizonServer.friendbot(keypair.publicKey()).call();

    printInfo('Keypair generated and funded');
    printInfo('Private key', keypair.secret());
    printInfo('Address', keypair.publicKey());
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(chain, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('generate-keypair').description('Generate keypair.');

    addBaseOptions(program, { ignorePrivateKey: true });

    program.addOption(new Option('--signatureScheme <signatureScheme>', 'sig scheme').choices(['ed25519']).default('ed25519'));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
