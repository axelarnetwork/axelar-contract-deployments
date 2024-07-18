'use strict';

require('dotenv').config();

const { Option } = require('commander');
const { getUnitAmount } = require('./amount-utils');

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
                .choices(['secp256k1', 'ed25519', 'secp256r1'])
                .default('secp256k1')
                .env('SIGNATURE_SCHEME'),
        );
    }

    if (options.address) {
        program.addOption(new Option('--address <address>', 'override contract address'));
    }

    return program;
};

const addExtendedOptions = (program, options = {}) => {
    addBaseOptions(program, options);

    if (options.contractName) {
        program.addOption(new Option('-c, --contractName <contractName>', 'contract name'));
    }

    return program;
};

// `optionMethod` is a method such as `addBaseOptions`
// `options` is an option object for optionMethod
const addOptionsToCommands = (program, optionMethod, options) => {
    if (program.commands.length > 0) {
        program.commands.forEach((command) => {
            optionMethod(command, options);
        });
    }

    optionMethod(program, options);
};

// Custom option processing for amount. https://github.com/tj/commander.js?tab=readme-ov-file#custom-option-processing
// The user is expected to pass a full amount (e.g. 1.0), and this option parser will convert it to smallest units (e.g. 1000000000).
// Note that this function will use decimals of 9 for SUI. So, other tokens with different decimals will not work.
const parseSuiUnitAmount = (value, previous) => {
    return getUnitAmount(value);
};

module.exports = {
    addBaseOptions,
    addExtendedOptions,
    addOptionsToCommands,
    parseSuiUnitAmount,
};
