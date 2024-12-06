'use strict';

require('dotenv').config();

const { Option, InvalidArgumentError } = require('commander');
const { getUnitAmount } = require('./amount-utils');
const { addEnvOption, addOptionsToCommands } = require('../../common');

const addBaseOptions = (program, options = {}) => {
    addEnvOption(program);
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(new Option('--gasOptions <gasOptions>', 'gas options cli override'));
    program.addOption(new Option('--chainName <chainName>', 'chainName').default('sui'));

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

    if (options.offline) {
        program.addOption(new Option('--sender <sender>', 'transaction sender'));
        program.addOption(new Option('--offline', 'store tx block for sign'));
        program.addOption(new Option('--txFilePath <file>', 'unsigned transaction will be stored'));
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

// Custom option processing for amount. https://github.com/tj/commander.js?tab=readme-ov-file#custom-option-processing
// The user is expected to pass a full amount (e.g. 1.0), and this option parser will convert it to smallest units (e.g. 1000000000).
// Note that this function will use decimals of 9 for SUI. So, other tokens with different decimals will not work.
const parseSuiUnitAmount = (value, previous) => {
    try {
        return getUnitAmount(value);
    } catch (error) {
        throw new InvalidArgumentError('Please use the correct format (e.g. 1.0)');
    }
};

module.exports = {
    addBaseOptions,
    addExtendedOptions,
    addOptionsToCommands,
    parseSuiUnitAmount,
};
