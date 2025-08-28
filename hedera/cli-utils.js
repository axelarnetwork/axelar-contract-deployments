'use strict';

require('dotenv').config();

const { Option } = require('commander');
const { printInfo } = require('../common/utils');

const addBaseOptions = (program, options = {}) => {
    program.addOption(
        new Option('-hn, --hederaNetwork <hederaNetworkName>', 'hedera network')
            .makeOptionMandatory(true)
            .choices(['mainnet', 'testnet', 'previewnet', 'local'])
            .env('HEDERA_NETWORK'),
    );
    program.addOption(new Option('-hid, --accountId <accountId>', 'account id').makeOptionMandatory(true).env('HEDERA_ID'));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'hex encoded private key').makeOptionMandatory(true).env('PRIVATE_KEY'));

    return program;
};

const addSkipPromptOption = (program, _options = {}) => {
    program.addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'));
    return program;
};

const printHederaNetwork = ({ hederaNetwork, accountId }) => {
    printInfo(`Using Hedera ${hederaNetwork}, Account ID ${accountId}`);
};

module.exports = {
    addBaseOptions,
    addSkipPromptOption,
    printHederaNetwork,
};
