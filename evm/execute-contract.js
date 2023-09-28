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
const { isNumber, isString, loadConfig, saveConfig, printObj, printLog, printError, printInfo } = require('./utils');

async function getCallData(methodName, targetContract, inputRecipient, inputAmount) {
    var recipient, amount;

    switch (methodName) {
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
            throw new Error('The method name does not match any of the specified choices');
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
        methodName,
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

    if (!isString(methodName)) {
        throw new Error('Missing method name from the user info.');
    }

    if (!isNumber(Number(nativeValue))) {
        throw new Error('Missing native value from user info');
    }

    var contractPath =
        callContractPath.charAt(0) === '@' ? callContractPath : callContractPath + callContractName + '.sol/' + callContractName + '.json';
    printInfo('Call Contract path', contractPath);

    const IContractExecutor = require(contractPath);
    const contract = new Contract(callContractAddress, IContractExecutor.abi, wallet);
    var finalCallData, finalNativeValue;

    if (methodName === 'default') {
        finalCallData = callData;
        finalNativeValue = nativeValue;
    } else {
        contractPath =
            targetContractPath.charAt(0) === '@'
                ? targetContractPath
                : targetContractPath + targetContractName + '.sol/' + targetContractName + '.json';
        printInfo('Target Contract path', contractPath);
        const ITargetContract = require(contractPath);
        const targetContract = new Contract(targetContractAddress, ITargetContract.abi, wallet);
        finalCallData = await getCallData(methodName, targetContract, recipientAddress, Number(amount));
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

const program = new Command();

program.name('execute-contract').description('Executes a call to an external contract');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(
    new Option('-pc, --callContractPath <callContractPath>', 'artifact path for the called contract').makeOptionMandatory(true),
);
program.addOption(new Option('-cn, --callContractName <callContractName>', 'name of the called contract').makeOptionMandatory(true));
program.addOption(
    new Option('-pt, --targetContractPath <targetContractPath>', 'artifact path for the target contract').makeOptionMandatory(true),
);
program.addOption(
    new Option(
        '-tn, --targetContractName <targetContractName>',
        'name of the target contract that is called through executeContract',
    ).makeOptionMandatory(false),
);
program.addOption(
    new Option('-ta, --targetContractAddress <targetContractAddress>', 'The address of the contract to be called')
        .makeOptionMandatory(true)
        .env('TARGET_ADDR'),
);
program.addOption(
    new Option('-v, --nativeValue <nativeValue>', 'The amount of native token (e.g., Ether) to be sent along with the call').default(0),
);
program.addOption(
    new Option('-m, --methodName <methodName>', 'method name to call in executeContract')
        .choices(['withdraw', 'transfer', 'approve', 'default'])
        .default('default'),
);
program.addOption(
    new Option('-k, --privateKey <privateKey>', 'The private key of the caller').makeOptionMandatory(true).env('PRIVATE_KEY'),
);
program.addOption(new Option('-c, --callData <callData>', 'The calldata to be sent').env('CALL_DATA').default('0x'));
program.addOption(new Option('-ra, --recipientAddress <recipientAddress>', 'The recipient address for the tokens').env('RECIPIENT_ADDR'));
program.addOption(new Option('-am, --amount <amount>', 'The amount of tokens to transfer/withdraw/provide allowance etc.').env('AMOUNT'));

program.action((options) => {
    main(options);
});

program.parse();
