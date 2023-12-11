'use strict';

require('dotenv').config();
const { ethers } = require('hardhat');
const {
    utils: { defaultAbiCoder, Interface },
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, printError, validateParameters, getContractJSON } = require('./utils');

const decode = (calldata, iface) => {
    try {
        const parsedCall = iface.parseTransaction({ data: calldata });
        const functionName = parsedCall.name;
        const functionFragment = iface.getFunction(functionName);

        const argNames = functionFragment.inputs.map((input) => input.name).join(', ');
        const argValues = parsedCall.args.map((arg) => arg.toString()).join(', ');

        return `\nFunction: ${functionName}\nArg names: ${argNames}\nArg values: ${argValues}`;
    } catch (error) {
        printError(`Unrecognized function call: ${calldata}`, error);
        return `\nFunction: Unrecognized function call`;
    }
};

const decodeMulticallData = (encodedData, contractJSON) => {
    const decodedArray = defaultAbiCoder.decode(['bytes[]'], encodedData)[0];
    const iface = new Interface(contractJSON.abi);

    return decodedArray.map((encodedCall) => {
        return decode(encodedCall, iface);
    });
};

function processCommand(options) {
    const { action, contractName, calldata } = options;

    validateParameters({ isNonEmptyString: { contractName }, isValidCalldata: { calldata } });

    printInfo('Contract name', contractName);

    printInfo('Action', action);

    const contractJSON = getContractJSON(contractName);

    switch (action) {
        case 'decode': {
            const iface = new Interface(contractJSON.abi);

            const decodedFunctionCall = decode(calldata, iface);

            printInfo('Decoded calldata', decodedFunctionCall);

            break;
        }

        case 'decodeMulticall': {
            const decodedMulticall = decodeMulticallData(calldata, contractJSON);

            printInfo('Decoded multicall data', decodedMulticall);

            break;
        }

        default: {
            throw new Error(`Unknown action ${action}`);
        }
    }
}

async function main(options) {
    processCommand(options);
}

if (require.main === module) {
    const program = new Command();

    program.name('Decode').description('Script to decode calldata');

    program.addOption(new Option('--action <action>', 'ITS action').choices(['decode', 'decodeMulticall']).makeOptionMandatory(true));

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
    program.addOption(new Option('--calldata <calldata>', 'calldata to decode').env('CALLDATA'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
