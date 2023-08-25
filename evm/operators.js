'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
    ContractFactory,
} = ethers;
const { Command, Option } = require('commander');

const { printInfo, printWalletInfo, loadConfig, saveConfig, isNumber, isAddressArray, isNumberArray, isKeccak256Hash } = require('./utils');

function getGasServiceInterface(contracts, wallet) {
    const gasServiceJson = require('../artifacts/contracts/gas-service/AxelarGasService.sol/AxelarGasService.json');
    const gasServiceFactory = new ContractFactory(gasServiceJson.abi, gasServiceJson.bytecode, wallet);
    const gasServiceContract = gasServiceFactory.attach(contracts[gasServiceJson.contractName].address);
    const gasServiceInterface = new ethers.utils.Interface(gasServiceContract.interface.fragments);

    return gasServiceInterface;
}

async function processCommand(options, chain, config) {
    const {
        artifactPath,
        contractName,
        operatorAction,
        privateKey,
        operatorAddress,
        receiver,
        tokens,
        amounts,
        txHash,
        logIndex,
        token,
        amount,
    } = options;

    if (contractName !== 'Operators') {
        throw new Error(`Invalid Operators contract: ${contractName}`);
    }

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

    if (contractConfig && !contractConfig.address) {
        throw new Error(`Contract ${contractName} is not deployed on ${chain}`);
    }

    if (operatorAddress && !isAddress(operatorAddress)) {
        throw new Error(`Invalid operator address: ${operatorAddress}.`);
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Contract name', contractName);

    const contractPath = artifactPath + contractName + '.sol/' + contractName + '.json';
    printInfo('Contract path', contractPath);

    const contractJson = require(contractPath);
    const operatorsFactory = new ContractFactory(contractJson.abi, contractJson.bytecode, wallet);
    const operatorsContract = operatorsFactory.attach(contractConfig.address);

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    printInfo('Operator Action', operatorAction);

    switch (operatorAction) {
        case 'isOperator': {
            if (!operatorAddress) {
                throw new Error('Operator address is mandatory for this action.');
            }

            const isOperator = await operatorsContract.isOperator(operatorAddress);
            console.log(`Is ${operatorAddress} an operator? ${isOperator}`);

            break;
        }

        case 'addOperator': {
            const owner = await operatorsContract.owner();

            if (owner.toLowerCase() !== wallet.address.toLowerCase()) {
                throw new Error(`Caller ${wallet.address} is not the contract owner.`);
            }

            if (!operatorAddress) {
                throw new Error('Operator address is mandatory for this action.');
            }

            let isOperator = await operatorsContract.isOperator(operatorAddress);

            if (isOperator) {
                throw new Error(`Address ${operatorAddress} is already an operator.`);
            }

            await operatorsContract.addOperator(operatorAddress, gasOptions).then((tx) => tx.wait());
            isOperator = await operatorsContract.isOperator(operatorAddress);

            if (!isOperator) {
                throw new Error('Add operator action failed.');
            } else {
                console.log(`Address ${operatorAddress} added as an operator.`);
            }

            break;
        }

        case 'removeOperator': {
            const owner = await operatorsContract.owner();

            if (owner.toLowerCase() !== wallet.address.toLowerCase()) {
                throw new Error(`Caller ${wallet.address} is not the contract owner.`);
            }

            if (!operatorAddress) {
                throw new Error('Operator address is mandatory for this action.');
            }

            let isOperator = await operatorsContract.isOperator(operatorAddress);

            if (!isOperator) {
                throw new Error(`Address ${operatorAddress} is not an operator.`);
            }

            await operatorsContract.removeOperator(operatorAddress, gasOptions).then((tx) => tx.wait());
            isOperator = await operatorsContract.isOperator(operatorAddress);

            if (isOperator) {
                throw new Error('Remove operator action failed.');
            } else {
                console.log(`Address ${operatorAddress} removed as an operator.`);
            }

            break;
        }

        case 'collectGas': {
            const isOperator = await operatorsContract.isOperator(wallet.address);

            if (!isOperator) {
                throw new Error(`Caller ${wallet.address} is not an operator.`);
            }

            if (!isAddress(receiver)) {
                throw new Error(`Invalid receiver address ${receiver}.`);
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

            const gasServiceInterface = getGasServiceInterface(contracts, wallet);
            const collectGasCalldata = gasServiceInterface.encodeFunctionData('collectFees', [receiver, tokens, amounts]);

            await operatorsContract.executeContract(target, collectGasCalldata, 0, gasOptions).then((tx) => tx.wait());

            break;
        }

        case 'refund': {
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
                throw new Error(`Invalid receiver address ${receiver}.`);
            }

            if (!isAddress(token)) {
                throw new Error(`Invalid token address.`);
            }

            if (!isNumber(amount)) {
                throw new Error('Invalid token amount.');
            }

            const target = chain.contracts.AxelarGasService?.address;

            if (!isAddress(target)) {
                throw new Error(`Missing AxelarGasService address in the chain info.`);
            }

            const gasServiceInterface = getGasServiceInterface(wallet);
            const refundCalldata = gasServiceInterface.encodeFunctionData('refund', [txHash, logIndex, receiver, token, amount]);

            await operatorsContract.executeContract(target, refundCalldata, 0, gasOptions).then((tx) => tx.wait());

            break;
        }

        default: {
            throw new Error(`Unknown operator action ${operatorAction}`);
        }
    }
}

async function main(options) {
    const config = loadConfig(options.env);

    const chain = options.chain;

    if (config.chains[chain.toLowerCase()] === undefined) {
        throw new Error(`Chain ${chain} is not defined in the info file`);
    }

    await processCommand(options, config.chains[chain.toLowerCase()], config);
    saveConfig(config, options.env);
}

const program = new Command();

program.name('operators-script').description('script to manage operators contract');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true));
program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
program.addOption(new Option('-n, --chain <chain>', 'chain name').makeOptionMandatory(true));
program.addOption(
    new Option('-o, --operatorAction <operatorAction>', 'operator action').choices([
        'isOperator',
        'addOperator',
        'removeOperator',
        'collectGas',
        'refund',
    ]),
);
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-d, --operatorAddress <operatorAddress>', 'operatorAddress').makeOptionMandatory(false));
program.addOption(new Option('-r, --receiver <receiver>', 'receiver address').makeOptionMandatory(false).env('RECEIVER'));
program.addOption(new Option('-ts, --tokens <tokens>', 'token addresses').makeOptionMandatory(false).env('TOKEN_ADDRESSES'));
program.addOption(new Option('-ms, --amounts <amounts>', 'token amounts').makeOptionMandatory(false).env('TOKEN_AMOUNTS'));
program.addOption(new Option('-x, --txHash <txHash>', 'tx hash').makeOptionMandatory(false).env('TX_HASH'));
program.addOption(new Option('-l, --logIndex <logIndex>', 'log index').makeOptionMandatory(false).env('LOG_INDEX'));
program.addOption(new Option('-t, --token <token>', 'token address').makeOptionMandatory(false).env('TOKEN_ADDRESS'));
program.addOption(new Option('-m, --amount <amount>', 'token amount').makeOptionMandatory(false).env('TOKEN_AMOUNT'));

program.action((options) => {
    main(options);
});

program.parse();
