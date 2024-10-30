'use strict';

const { Command, Option } = require('commander');
const { addBaseOptions, generateKeypair, getRawPrivateKey } = require('./utils');
const { saveConfig, loadConfig, printInfo, getChainConfig } = require('../common/utils');

const { ethers } = require('hardhat');
const { hexlify } = ethers.utils;

async function processCommand(config, chain, options) {
    const keypair = await generateKeypair(options);

    printInfo('Keypair generated');
    printInfo('Private key', keypair.getSecretKey());
    printInfo('Private key hex', hexlify(getRawPrivateKey(keypair)));
    printInfo('Public key', hexlify(keypair.getPublicKey().toRawBytes()));
    printInfo('Address', keypair.toSuiAddress());
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(config, chain, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('generate-keypair').description('Generate keypair.');

    addBaseOptions(program, { ignorePrivateKey: true });

    program.addOption(
        new Option('--signatureScheme <signatureScheme>', 'sig scheme').choices(['ed25519', 'secp256k1', 'secp256r1']).default('ed25519'),
    );

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
