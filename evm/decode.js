'use strict';

require('dotenv').config();
const { ethers } = require('hardhat');
const {
    utils: { Interface },
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, printError, validateParameters, getContractJSON } = require('./utils');

const decode = (calldata, iface) => {
    try {
        const parsedCall = iface.parseTransaction({ data: calldata });
        const functionName = parsedCall.name;
        const functionFragment = iface.getFunction(functionName);

        if (functionName === 'multicall') {
            const data = parsedCall.args[0];
            return `\nFunction: multicall\nDecoded multicall:${decodeMulticallData(data, iface)}`;
        }

        const argNames = functionFragment.inputs.map((input) => input.name).join(', ');
        const argValues = parsedCall.args.map((arg) => arg.toString()).join(', ');

        return `\nFunction: ${functionName}\nArg names: ${argNames}\nArg values: ${argValues}`;
    } catch (error) {
        printError(`Unrecognized function call: ${calldata}`, error);
        return `\nFunction: Unrecognized function call`;
    }
};

const decodeMulticallData = (encodedData, iface) => {
    return encodedData.map((encodedCall) => {
        return decode(encodedCall, iface);
    });
};

function processCommand(options) {
    const { contractName, calldata } = options;

    validateParameters({ isNonEmptyString: { contractName }, isValidCalldata: { calldata } });

    printInfo('Contract name', contractName);

    const contractJSON = getContractJSON(contractName);
    const iface = new Interface(contractJSON.abi);

    const decodedFunctionCall = decode(calldata, iface);

    printInfo('Decoded calldata', decodedFunctionCall);
}

async function main(options) {
    processCommand(options);
}

if (require.main === module) {
    const program = new Command();

    program.name('Decode').description('Script to decode calldata');

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
    program.addOption(new Option('--calldata <calldata>', 'calldata to decode').env('CALLDATA'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
