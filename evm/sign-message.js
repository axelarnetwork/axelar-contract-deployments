'use strict';

const { ethers } = require('hardhat');
const {
    utils: { isHexString, arrayify },
} = ethers;
const { Command, Option } = require('commander');
const { getWallet } = require('./sign-utils');
const { printInfo, validateParameters } = require('./utils');

async function processCommand(options) {
    const { message, privateKey } = options;

    validateParameters({ isValidPrivateKey: { privateKey }, isNonEmptyString: { message } });

    const wallet = await getWallet(privateKey);
    printInfo('Wallet address', await wallet.getAddress());
    let sig;

    if (isHexString(message) && message.length === 66) {
        const messageHashBytes = arrayify(message);
        // Sign the binary data
        sig = await wallet.signMessage(messageHashBytes);
    } else {
        // Sign the string message
        sig = await wallet.signMessage(message);
    }

    printInfo('The signed message is:', sig);
}

async function main(options) {
    await processCommand(options);
}

if (require.main === module) {
    const program = new Command();

    program.name('sign-message').description('sign a message from the user wallet');

    program.addOption(new Option('-m, --message <message>', 'the message to be signed').makeOptionMandatory(true).env('MESSAGE'));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
