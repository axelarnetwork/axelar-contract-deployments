'use strict';

require('dotenv').config();
const { Relayer } = require('@openzeppelin/defender-relay-client');
const { ethers } = require('hardhat');
const {
    utils: { hexlify, joinSignature, toUtf8Bytes },
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, validateParameters, printError } = require('./utils');

async function main(options) {
    const { apiKey, secret, message } = options;

    validateParameters({ isNonEmptyString: { apiKey, secret, message } });

    try {
        const relayer = new Relayer({ apiKey, apiSecret: secret });
        const messageHex = hexlify(toUtf8Bytes(message));

        const compactSignature = await relayer.sign({ message: messageHex });
        const signature = joinSignature(compactSignature);

        printInfo('Signature', signature);
    } catch (error) {
        printError('Error while signing message', error.message);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('sign message from defender relayer').description("generate signed message using defender's api.");

    program.addOption(
        new Option('-k, --apiKey <api key>', 'api key of defender-relay-client').makeOptionMandatory(true).env('DEFENDER_API_KEY'),
    );
    program.addOption(
        new Option('-s, --secret <secret>', 'secret of api key of defender-relay-client').makeOptionMandatory(true).env('DEFENDER_SECRET'),
    );
    program.addOption(new Option('-m, --message <message>', 'message to be signed').makeOptionMandatory(true));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
