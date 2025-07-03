'use strict';

require('dotenv').config();

const { Option } = require('commander');

const addBaseOptions = (program, options = {}) => {
    program.addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'));

    program.addOption(new Option('-hn, --hederaNetwork <hederaNetworkName>', 'hedera network').makeOptionMandatory(true).env('HEDERA_NETWORK'));
    program.addOption(new Option('-hid, --accountId <accountId>', 'account id').makeOptionMandatory(true).env('HEDERA_ID'));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'hex encoded private key').makeOptionMandatory(true).env('PRIVATE_KEY'));

    return program;
};


module.exports = {
    addBaseOptions,
};
