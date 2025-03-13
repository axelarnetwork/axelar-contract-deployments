'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { isAddress, Interface },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    printError,
    printWalletInfo,
    isAddressArray,
    isNumberArray,
    parseArgs,
    prompt,
    getGasOptions,
    mainProcessor,
    validateParameters,
    getContractJSON,
    printWarn,
} = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { getGasUpdates, printFailedChainUpdates, addFailedChainUpdate, relayTransaction } = require('./gas-service');
const { getWallet } = require('./sign-utils');

async function processCommand(config, chain, options) {
    const {
        env,
        contractName,
        address,
        action,
        privateKey,
        args,

        chains,

        yes,
    } = options;

    const argsArray = args ? parseArgs(args) : [];

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

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

    const wallet = await getWallet(privateKey, provider, options);
    await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);

    const operatorsContract = new Contract(operatorsAddress, getContractJSON('IOperators').abi, wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

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
            printInfo(`Is ${operatorAddress} an operator on ${chain.name}?`, `${isOperator}`);

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

            const isOperator = await operatorsContract.isOperator(operatorAddress);

            if (isOperator) {
                printError(`Address ${operatorAddress} is already an operator.`);
                return;
            }

            await operatorsContract.addOperator(operatorAddress, gasOptions).then((tx) => tx.wait());

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

            const gasServiceInterface = new Interface(getContractJSON('IAxelarGasService').abi);
            const collectFeesCalldata = gasServiceInterface.encodeFunctionData('collectFees', [receiver, tokens, amounts]);

            try {
                await operatorsContract.executeContract(target, collectFeesCalldata, 0, gasOptions).then((tx) => tx.wait());
            } catch (error) {
                printError(error);
            }

            break;
        }

        case 'refund': {
            const [txHash, logIndex, receiver, token, amount] = argsArray;

            const isOperator = await operatorsContract.isOperator(wallet.address);

            if (!isOperator) {
                throw new Error(`Caller ${wallet.address} is not an operator.`);
            }

            const target = chain.contracts.AxelarGasService?.address;

            validateParameters({
                isKeccak256Hash: { txHash },
                isNumber: { logIndex, amount },
                isAddress: { receiver, token, target },
            });

            const gasServiceInterface = new Interface(getContractJSON('IAxelarGasService').abi);
            const refundCalldata = gasServiceInterface.encodeFunctionData('refund', [txHash, logIndex, receiver, token, amount]);

            try {
                await operatorsContract.executeContract(target, refundCalldata, 0, gasOptions).then((tx) => tx.wait());
            } catch (error) {
                printError(error);
            }

            break;
        }

        case 'updateGasInfo': {
            const target = chain.contracts.AxelarGasService?.address;

            validateParameters({
                isNonEmptyStringArray: { chains },
                isAddress: { target },
            });

            const { chainsToUpdate, gasInfoUpdates } = await getGasUpdates(config, env, chain, chains);

            if (chainsToUpdate.length === 0) {
                printWarn('No gas info updates found.');
                return;
            }

            printInfo('Collected gas info for the following chain names', chainsToUpdate.join(', '));

            if (prompt(`Submit gas update transaction?`, yes)) {
                return;
            }

            const gasServiceInterface = new Interface(getContractJSON('IAxelarGasService').abi);
            const updateGasInfoCalldata = gasServiceInterface.encodeFunctionData('updateGasInfo', [chainsToUpdate, gasInfoUpdates]);

            try {
                await relayTransaction(
                    options,
                    chain,
                    operatorsContract,
                    'executeContract',
                    [target, updateGasInfoCalldata, 0],
                    0,
                    gasOptions,
                );
            } catch (error) {
                for (let i = 0; i < chainsToUpdate.length; i++) {
                    addFailedChainUpdate(chain.name, chainsToUpdate[i]);
                }

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
    await mainProcessor(options, processCommand);

    printFailedChainUpdates();
}

if (require.main === module) {
    const program = new Command();

    program.name('operators').description('script to manage operators contract');

    addBaseOptions(program, { address: true });

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').default('Operators'));
    program.addOption(
        new Option('--action <action>', 'operator action').choices([
            'isOperator',
            'addOperator',
            'removeOperator',
            'collectFees',
            'refund',
            'updateGasInfo',
        ]),
    );
    program.addOption(new Option('--offline', 'run script in offline mode'));
    program.addOption(new Option('--args <args>', 'operator action arguments'));

    // options for updateGasInfo
    program.addOption(new Option('--chains <chains...>', 'Chain names'));
    program.addOption(new Option('--relayerAPI <relayerAPI>', 'Relay the tx through an external relayer API').env('RELAYER_API'));
    program.addOption(new Option('--ignoreError', 'Ignore errors and proceed to next chain'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
