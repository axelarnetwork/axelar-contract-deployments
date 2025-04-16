'use strict';

const { Command } = require('commander');
const { addBaseOptions } = require('./utils');
const { loadConfig, printInfo, getChainConfig } = require('../common/utils');
const { Horizon, Keypair } = require('@stellar/stellar-sdk');

async function processCommand(chain, _options) {
    const keypair = Keypair.random();
    const horizonServer = new Horizon.Server(chain.horizonRpc);

    printInfo('Keypair generated');
    printInfo('Private key', keypair.secret());
    printInfo('Address', keypair.publicKey());

    // Initializes the account on-chain; without this call, the account does not yet exist.
    // Friendbot funds and activates the account (only available on testnets)
    await horizonServer.friendbot(keypair.publicKey()).call();
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

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
