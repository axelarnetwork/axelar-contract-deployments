'use strict';

const { printObj, readJSON, writeJSON, importNetworks, verifyContract, getBytecodeHash } = require('./utils');
const { deployITS } = require('./deploy-its');
const { deployAmplifierGateway } = require('./deploy-amplifier-gateway');
const { deployGateway } = require('./deploy-gateway-v6.2.x');

module.exports = {
    printObj,
    readJSON,
    writeJSON,
    importNetworks,
    verifyContract,
    getBytecodeHash,
    deployITS,
    deployAmplifierGateway,
    deployGateway,
};
