'use strict';

const { printObj, readJSON, writeJSON, importNetworks, verifyContract, getBytecodeHash } = require('./utils');
const { deployITS } = require('./deploy-its');
const { deployAmplifierGateway } = require('./deploy-amplifier-gateway');
const { deployLegacyGateway } = require('./deploy-consensus-gateway');

module.exports = {
    printObj,
    readJSON,
    writeJSON,
    importNetworks,
    verifyContract,
    getBytecodeHash,
    deployITS,
    deployAmplifierGateway,
    deployLegacyGateway,
};
