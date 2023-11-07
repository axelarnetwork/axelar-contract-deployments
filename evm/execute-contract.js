'use strict';

require('dotenv').config();

const ethers = require('hardhat');
const {
    Wallet,
    Contract,
    getDefaultProvider,
    utils: { isAddress },
} = ethers;

const readlineSync = require('readline-sync');
const { Command, Option } = require('commander');
const { isNumber, isString, loadConfig, saveConfig, printObj, printLog, printError, getContractJSON } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

async function getCallData(action, targetContract, inputRecipient, inputAmount) {
    var recipient, amount;

    switch (action) {
        case 'withdraw': {
            if (inputRecipient) {
                recipient = inputRecipient;
            } else {
                recipient = readlineSync.question('Enter the recipient address for withdrawal:');
            }

            if (inputAmount) {
                amount = inputAmount;
            } else {
                amount = readlineSync.question('Enter the amount of tokens to withdraw:');
            }

            if (!isAddress(recipient)) {
                throw new Error('Missing or incorrect recipient address for withdrawal from user input.');
            }

            if (!isNumber(amount)) {
                throw new Error('Missing or incorrect withdrawal amount from user input.');
            }

            const callData = targetContract.interface.encodeFunctionData('withdraw', [recipient, amount]);
            return callData;
        }

        case 'transfer': {
            if (inputRecipient) {
                recipient = inputRecipient;
            } else {
                recipient = readlineSync.question('Enter the recipient address for transfer:');
            }

            if (inputAmount) {
                amount = inputAmount;
            } else {
                amount = readlineSync.question('Enter the amount of tokens to transfer:');
            }

            if (!isAddress(recipient)) {
                throw new Error('Missing or incorrect recipient address for transfer from user input.');
            }

            if (!isNumber(amount)) {
                throw new Error('Missing or incorrect transfer amount from user input.');
            }

            const callData = targetContract.interface.encodeFunctionData('transfer', [recipient, amount]);
            return callData;
        }

        case 'approve': {
            if (inputRecipient) {
                recipient = inputRecipient;
            } else {
                recipient = readlineSync.question('Enter the recipient address for approval:');
            }

            if (inputAmount) {
                amount = inputAmount;
            } else {
                amount = readlineSync.question('Enter the amount of tokens to approve:');
            }

            if (!isAddress(recipient)) {
                throw new Error('Missing or incorrect recipient address for approval from user input.');
            }

            if (!isNumber(amount)) {
                throw new Error('Missing or incorrect approval amount from user input.');
            }

            const callData = targetContract.interface.encodeFunctionData('approve', [recipient, amount]);
            return callData;
        }

        default: {
            throw new Error('The action does not match any of the specified choices');
        }
    }
}

async function executeContract(options, chain, wallet) {
    const {
        callContractPath,
        targetContractPath,
        callContractName,
        targetContractName,
        targetContractAddress,
        callData,
        nativeValue,
        action,
        recipientAddress,
        amount,
    } = options;
    const contracts = chain.contracts;

    if (!contracts[callContractName]) {
        throw new Error('Missing call contract address in the info file');
    }

    const callContractAddress = contracts[callContractName].address;

    if (!isAddress(callContractAddress)) {
        throw new Error('Missing call contract address in the address info.');
    }

    if (!isAddress(targetContractAddress)) {
        throw new Error('Missing target address in the address info.');
    }

    if (!isString(action)) {
        throw new Error('Missing method name from the user info.');
    }

    if (!isNumber(Number(nativeValue))) {
        throw new Error('Missing native value from user info');
    }

    const IContractExecutor = getContractJSON(callContractName, callContractPath);
    const contract = new Contract(callContractAddress, IContractExecutor.abi, wallet);
    var finalCallData, finalNativeValue;

    if (action === 'default') {
        finalCallData = callData;
        finalNativeValue = nativeValue;
    } else {
        const ITargetContract = getContractJSON(targetContractName, targetContractPath);
        const targetContract = new Contract(targetContractAddress, ITargetContract.abi, wallet);
        finalCallData = await getCallData(action, targetContract, recipientAddress, Number(amount));
        finalNativeValue = Number(0);
    }

    (async () => {
        try {
            const result = await contract.executeContract(targetContractAddress, finalCallData, finalNativeValue);
            printLog('Function successfully called with return value as: ');
            printObj(result);
        } catch (error) {
            printError('Calling executeContract method failed with Error: ', error);
        }
    })();
}

async function main(options) {
    const config = loadConfig(options.env);
    let chains = options.chainNames.split(',').map((str) => str.trim());

    if (options.chainNames === 'all') {
        chains = Object.keys(config.chains);
    }

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chain} is not defined in the info file`);
        }
    }

    for (const chain of chains) {
        const rpc = config.chains[chain.toLowerCase()].rpc;
        const provider = getDefaultProvider(rpc);
        const privateKey = options.privateKey;

        if (!isString(privateKey)) {
            throw new Error('Private Key value is not provided in the info file');
        }

        const wallet = new Wallet(privateKey, provider);
        await executeContract(options, config.chains[chain.toLowerCase()], wallet);
        saveConfig(config, options.env);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('execute-contract').description('Executes a call to an external contract');

    addBaseOptions(program);

    program.addOption(new Option('-c, --callContractName <callContractName>', 'name of the called contract').makeOptionMandatory(true));
    program.addOption(new Option('--callContractPath <callContractPath>', 'artifact path for the called contract'));
    program.addOption(new Option('--targetContractPath <targetContractPath>', 'artifact path for the target contract'));
    program.addOption(
        new Option('-t, --targetContractName <targetContractName>', 'target contract name called by executeContract').makeOptionMandatory(
            true,
        ),
    );
    program.addOption(
        new Option('-a, --targetContractAddress <targetContractAddress>', 'target contract address')
            .makeOptionMandatory(true)
            .env('TARGET_ADDR'),
    );
    program.addOption(
        new Option('-v, --nativeValue <nativeValue>', 'The amount of native token (e.g., Ether) to be sent along with the call').default(0),
    );
    program.addOption(
        new Option('--action <action>', 'executeContract action')
            .choices(['withdraw', 'transfer', 'approve', 'default'])
            .default('default'),
    );
    program.addOption(new Option('--callData <callData>', 'The calldata to be sent').env('CALL_DATA').default('0x'));
    program.addOption(new Option('--recipientAddress <recipientAddress>', 'The recipient address for the tokens').env('RECIPIENT_ADDR'));
    program.addOption(new Option('--amount <amount>', 'The amount of tokens to transfer/withdraw/provide allowance etc.').env('AMOUNT'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
