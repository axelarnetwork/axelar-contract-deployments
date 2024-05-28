'use strict';

require('dotenv').config();

const { Option } = require('commander');

const addBaseOptions = (program, options = {}) => {
    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'devnet-amplifier', 'devnet-verifiers', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(new Option('--gasOptions <gasOptions>', 'gas options cli override'));

    if (!options.ignorePrivateKey) {
        program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));

        program.addOption(
            new Option('--privateKeyType <privateKeyType>', 'private key type')
                .makeOptionMandatory(true)
                .choices(['bech32', 'mnemonic', 'hex'])
                .default('bech32')
                .env('PRIVATE_KEY_TYPE'),
        );

        program.addOption(
            new Option('--signatureScheme <signatureScheme>', 'signature scheme to use')
                .choices(['ed25519', 'secp256k1', 'secp256r1'])
                .default('ed25519')
                .env('SIGNATURE_SCHEME'),
        );
    }

    if (options.address) {
        program.addOption(new Option('--address <address>', 'override contract address'));
    }

    return program;
};

module.exports = {
    addBaseOptions,
};
