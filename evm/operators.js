'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress, Interface },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    printError,
    printWalletInfo,
    loadConfig,
    isNumber,
    isAddressArray,
    isNumberArray,
    isKeccak256Hash,
    parseArgs,
    prompt,
    addCallContractOptions,
} = require('./utils');
const IAxelarGasService = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGasService.json');
const IOperators = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IOperators.json');

async function processCommand(options, chain) {
    const { contractName, address, action, privateKey, args, yes } = options;

    const argsArray = parseArgs(args);

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

    printInfo('Chain', chain.name);

    let operatorsAddress;

    if (isAddress(address)) {
        operatorsAddress = address;
    } else {
        if (!contractConfig?.address) {
            throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
        }

        operatorsAddress = contractConfig.address;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Contract name', contractName);

    const operatorsContract = new Contract(operatorsAddress, IOperators.abi, wallet);

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    printInfo('Operator Action', action);

    if (prompt(`Proceed with ${action} on ${chain.name}?`, yes)) {
        return;
    }

    switch (action) {
        case 'isOperator': {
            const operatorAddress = argsArray[0];

            if (!isAddress(operatorAddress)) {
                throw new Error(`Invalid operator address: ${operatorAddress}.`);
            }

            const isOperator = await operatorsContract.isOperator(operatorAddress);
            printInfo(`Is ${operatorAddress} an operator?`, `${isOperator}`);

            break;
        }

        case 'addOperator': {
            const operatorAddress = argsArray[0];
            const owner = await operatorsContract.owner();

            if (owner.toLowerCase() !== wallet.address.toLowerCase()) {
                throw new Error(`Caller ${wallet.address} is not the contract owner.`);
            }

            if (!isAddress(operatorAddress)) {
                throw new Error(`Invalid operator address: ${operatorAddress}`);
            }

            let isOperator = await operatorsContract.isOperator(operatorAddress);

            if (isOperator) {
                throw new Error(`Address ${operatorAddress} is already an operator.`);
            }

            await operatorsContract.addOperator(operatorAddress, gasOptions).then((tx) => tx.wait());
            isOperator = await operatorsContract.isOperator(operatorAddress);

            if (!isOperator) {
                throw new Error('Add operator action failed.');
            }

            printInfo(`Address ${operatorAddress} added as an operator.`);

            break;
        }

        case 'removeOperator': {
            const operatorAddress = argsArray[0];
            const owner = await operatorsContract.owner();

            if (owner.toLowerCase() !== wallet.address.toLowerCase()) {
                throw new Error(`Caller ${wallet.address} is not the contract owner.`);
            }

            if (!isAddress(operatorAddress)) {
                throw new Error(`Invalid operator address: ${operatorAddress}`);
            }

            let isOperator = await operatorsContract.isOperator(operatorAddress);

            if (!isOperator) {
                throw new Error(`Address ${operatorAddress} is not an operator.`);
            }

            await operatorsContract.removeOperator(operatorAddress, gasOptions).then((tx) => tx.wait());
            isOperator = await operatorsContract.isOperator(operatorAddress);

            if (isOperator) {
                throw new Error('Remove operator action failed.');
            }

            printInfo(`Address ${operatorAddress} removed as an operator.`);

            break;
        }

        case 'collectFees': {
            const receiver = argsArray[0];
            const tokens = argsArray[1];
            const amounts = argsArray[2];

            const isOperator = await operatorsContract.isOperator(wallet.address);

            if (!isOperator) {
                throw new Error(`Caller ${wallet.address} is not an operator.`);
            }

            if (!isAddress(receiver)) {
                throw new Error(`Invalid receiver address: ${receiver}`);
            }

            if (!isAddressArray(tokens)) {
                throw new Error(`Invalid token addresses.`);
            }

            if (!isNumberArray(amounts)) {
                throw new Error('Invalid token amounts.');
            }

            if (tokens.length !== amounts.length) {
                throw new Error('Token addresses and token amounts have a length mismatch.');
            }

            const target = chain.contracts.AxelarGasService?.address;

            if (!isAddress(target)) {
                throw new Error(`Missing AxelarGasService address in the chain info.`);
            }

            const gasServiceInterface = new Interface(IAxelarGasService.abi);
            const collectFeesCalldata = gasServiceInterface.encodeFunctionData('collectFees', [receiver, tokens, amounts]);

            try {
                await operatorsContract.executeContract(target, collectFeesCalldata, 0, gasOptions).then((tx) => tx.wait());
            } catch (error) {
                printError(error);
            }

            break;
        }

        case 'refund': {
            const txHash = argsArray[0];
            const logIndex = argsArray[1];
            const receiver = argsArray[2];
            const token = argsArray[3];
            const amount = argsArray[4];

            const isOperator = await operatorsContract.isOperator(wallet.address);

            if (!isOperator) {
                throw new Error(`Caller ${wallet.address} is not an operator.`);
            }

            if (!isKeccak256Hash(txHash)) {
                throw new Error(`Invalid tx hash: ${txHash}`);
            }

            if (!isNumber(logIndex)) {
                throw new Error(`Invalid log index: ${logIndex}`);
            }

            if (!isAddress(receiver)) {
                throw new Error(`Invalid receiver address: ${receiver}`);
            }

            if (!isAddress(token)) {
                throw new Error(`Invalid token address: ${token}`);
            }

            if (!isNumber(amount)) {
                throw new Error(`Invalid token amount: ${amount}`);
            }

            const target = chain.contracts.AxelarGasService?.address;

            if (!isAddress(target)) {
                throw new Error(`Missing AxelarGasService address in the chain info.`);
            }

            const gasServiceInterface = new Interface(IAxelarGasService.abi);
            const refundCalldata = gasServiceInterface.encodeFunctionData('refund', [txHash, logIndex, receiver, token, amount]);

            try {
                await operatorsContract.executeContract(target, refundCalldata, 0, gasOptions).then((tx) => tx.wait());
            } catch (error) {
                printError(error);
            }

            break;
        }

        default: {
            throw new Error(`Unknown operator action: ${action}`);
        }
    }
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
        await processCommand(options, config.chains[chain.toLowerCase()]);
    }
}

const program = new Command();

program.name('operators').description('script to manage operators contract');

addCallContractOptions(program);

program.addOption(new Option('-c, --contractName <contractName>', 'contract name').default('Operators').makeOptionMandatory(false));
program.addOption(
    new Option('--action <action>', 'operator action').choices(['isOperator', 'addOperator', 'removeOperator', 'collectFees', 'refund']),
);
program.addOption(new Option('--args <args>', 'operator action arguments').makeOptionMandatory(true));

program.action((options) => {
    main(options);
});

program.parse();
