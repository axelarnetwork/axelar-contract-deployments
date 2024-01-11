'use strict';

const { ethers } = require('hardhat');
const {
    utils: { splitSignature, isHexString, arrayify },
} = ethers;
const { Command, Option } = require('commander');
const { getWallet } = require('./sign-utils');
const { printInfo } = require('./utils');

async function processCommand(options) {
    const { message, privateKey } = options;

    const wallet = await getWallet(privateKey, '');
    // let wallet = new Wallet('9c6cd64132c4bb5c9e812638c2865e78fa789d73a7d934500e3fbd22c68bbe4a', "");
    printInfo('Wallet address', await wallet.getAddress());
    let flatSig;

    if (isHexString(message) && message.length === 66) {
        const messageHashBytes = arrayify(message);
        // Sign the binary data
        flatSig = await wallet.signMessage(messageHashBytes);
    } else {
        // Sign the string message
        flatSig = await wallet.signMessage(message);
    }

    // For Solidity, we need the expanded-format of a signature
    const sig = splitSignature(flatSig);
    console.log('The signed message is: \n', sig);
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
