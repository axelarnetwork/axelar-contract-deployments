const { Command, Option } = require('commander');
const { mainProcessor, deriveAddress } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');
const { printInfo } = require('../common');

async function rotateSigners(_config, wallet, client, chain, options) {
    // Wallet must be the initial signer of the multisig account,
    // with enough weight to reach quorum.
    const multisig = chain.contracts.AxelarGateway.address;

    if (options.signerPublicKeys.length !== options.signerWeights.length) {
        throw new Error('Number of signer public keys must match number of signer weights');
    }

    printInfo('Updating multisig signer set');
    await client.sendSignerListSet(
        wallet,
        {
            account: multisig,
            quorum: Number(options.quorum),
            signers: options.signerPublicKeys.map((signedPubKey, i) => ({
                address: deriveAddress(signedPubKey),
                weight: Number(options.signerWeights[i]),
            })),
        },
        { multisign: true, ...options },
    );

    printInfo('Successfully rotated signers');
}

if (require.main === module) {
    const program = new Command();

    program
        .name('rotate-signers')
        .description('Rotate signers of the XRPL multisig account.')
        .addOption(new Option('--signerPublicKeys <signerPublicKeys...>', 'public keys of the new signers').makeOptionMandatory(true))
        .addOption(new Option('--signerWeights <signerWeights...>', 'weights of the new signers').makeOptionMandatory(true))
        .addOption(new Option('--quorum <quorum>', 'new quorum for the multisig account').makeOptionMandatory(true));

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.action((options) => {
        mainProcessor(rotateSigners, options);
    });

    program.parse();
}
