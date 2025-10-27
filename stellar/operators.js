'use strict';

const { Contract, nativeToScVal } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { getWallet, broadcast, addBaseOptions, addressToScVal, tokenToScVal, isValidAddress } = require('./utils');
const {
    loadConfig,
    printInfo,
    printWarn,
    parseArgs,
    validateParameters,
    saveConfig,
    addOptionsToCommands,
    getChainConfig,
} = require('../common');
const { prompt } = require('../common/utils');

async function isOperator(wallet, _, chain, contract, args, options) {
    const [address] = args;

    validateParameters({
        isValidStellarAddress: { address },
    });

    const operation = contract.call('is_operator', addressToScVal(address));
    const isOperator = await broadcast(operation, wallet, chain, 'is_operator called', options);

    if (isOperator.value()) {
        printInfo(address + ' is an operator');
    } else {
        printWarn(address + ' is not an operator');
    }
}

async function addOperator(wallet, _, chain, contract, args, options) {
    const [address] = args;

    validateParameters({
        isValidStellarAddress: { address },
    });

    const operation = contract.call('add_operator', addressToScVal(address));
    await broadcast(operation, wallet, chain, 'add_operator called', options);
}

async function removeOperator(wallet, _, chain, contract, args, options) {
    const [address] = args;

    validateParameters({
        isValidStellarAddress: { address },
    });

    const operation = contract.call('remove_operator', addressToScVal(address));
    await broadcast(operation, wallet, chain, 'remove_operator called', options);
}

async function collectFees(wallet, _, chain, contract, args, options) {
    const operator = addressToScVal(wallet.publicKey());
    const [receiver] = args;
    const gasServiceAddress = chain.contracts?.AxelarGasService?.address;
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;
    const gasAmount = options.gasAmount;

    validateParameters({
        isNonEmptyString: { receiver },
        isValidStellarAddress: { gasServiceAddress, gasTokenAddress },
        isValidNumber: { gasAmount },
    });

    const target = addressToScVal(gasServiceAddress);
    const method = nativeToScVal('collect_fees', { type: 'symbol' });
    const params = nativeToScVal([addressToScVal(receiver), tokenToScVal(gasTokenAddress, gasAmount)]);

    const operation = contract.call('execute', operator, target, method, params);

    await broadcast(operation, wallet, chain, 'collect_fees called', options);
}

async function refund(wallet, _, chain, contract, args, options) {
    const operator = addressToScVal(wallet.publicKey());
    const [messageId, receiver] = args;
    const gasServiceAddress = chain.contracts?.AxelarGasService?.address;
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;
    const gasAmount = options.gasAmount;

    validateParameters({
        isNonEmptyString: { messageId, receiver },
        isValidStellarAddress: { gasServiceAddress, gasTokenAddress },
        isValidNumber: { gasAmount },
    });

    const target = addressToScVal(gasServiceAddress);
    const method = nativeToScVal('refund', { type: 'symbol' });
    const params = nativeToScVal([
        nativeToScVal(messageId, { type: 'string' }),
        addressToScVal(receiver),
        tokenToScVal(gasTokenAddress, gasAmount),
    ]);

    const operation = contract.call('execute', operator, target, method, params);

    await broadcast(operation, wallet, chain, 'refund called', options);
}

async function execute(wallet, _, chain, contract, args, options) {
    const operator = addressToScVal(wallet.publicKey());
    const [target, method, params] = args;

    validateParameters({
        isNonEmptyString: { target, method, params },
    });

    const operation = contract.call(
        'execute',
        operator,
        addressToScVal(target),
        nativeToScVal(method, { type: 'symbol' }),
        nativeToScVal(parseArgs(params || '')),
    );

    await broadcast(operation, wallet, chain, 'Executed', options);
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

    const contractAddress = chain.contracts?.AxelarOperators?.address;

    validateParameters({
        isValidStellarAddress: { contractAddress },
    });

    if (!isValidAddress(contractAddress)) {
        throw new Error('Invalid operators contract');
    }

    const contract = new Contract(contractAddress);

    await processor(wallet, config, chain, contract, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('operators').description('Operators contract management');

    program.command('is-operator <address>').action((address, options) => {
        mainProcessor(isOperator, [address], options);
    });

    program.command('add-operator <address>').action((address, options) => {
        mainProcessor(addOperator, [address], options);
    });

    program.command('remove-operator <address>').action((address, options) => {
        mainProcessor(removeOperator, [address], options);
    });

    program
        .command('collect-fees <receiver>')
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default(0))
        .action((receiver, options) => {
            mainProcessor(collectFees, [receiver], options);
        });

    program
        .command('refund <messageId> <receiver>')
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default(0))
        .action((messageId, receiver, options) => {
            mainProcessor(refund, [messageId, receiver], options);
        });

    program.command('execute <target> <method> <params>').action((target, method, params, options) => {
        mainProcessor(execute, [target, method, params], options);
    });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
