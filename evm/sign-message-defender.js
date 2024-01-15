const { Relayer } = require('@openzeppelin/defender-relay-client');
const { ethers } = require('ethers');
const { Command, Option } = require('commander');
const { printInfo } = require('./utils');

async function main(options) {
    const { apiKey, secret, message } = options;

    const relayer = new Relayer({ apiKey, apiSecret: secret });
    const messageHex = ethers.utils.hexlify(ethers.utils.toUtf8Bytes(message));

    const compactSignature = await relayer.sign({ message: messageHex });
    const signature = ethers.utils.joinSignature(compactSignature);

    printInfo('Signature:', signature);
}

if (require.main === module) {
    const program = new Command();

    program.name('sign message from defender relayer').description("generate signed message using defender's api.");

    program.addOption(new Option('-k, --apiKey <api key>', 'api key of defender-relay-client').makeOptionMandatory(true));
    program.addOption(new Option('-s, --secret <secret>', 'secret of api key of defender-relay-client').makeOptionMandatory(true));
    program.addOption(new Option('-m, --message <message>', 'message to be signed').makeOptionMandatory(true));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
