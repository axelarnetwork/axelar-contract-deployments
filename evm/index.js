'use strict';

const { printObj, readJSON, writeJSON, importNetworks, verifyContract, getBytecodeHash } = require('./utils');
const { deployGatewayv43 } = require('./deploy-gateway-v4.3.x');
const { deployGatewayv5 } = require('./deploy-gateway-v5.x');

module.exports = {
    printObj,
    readJSON,
    writeJSON,
    importNetworks,
    verifyContract,
    getBytecodeHash,
    deployGatewayv43,
    deployGatewayv5,
};
