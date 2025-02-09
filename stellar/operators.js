'use strict';

const { Address, Contract } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const { loadConfig, printInfo, printWarn, parseArgs, validateParameters, saveConfig } = require('../evm/utils');
const { getWallet, broadcast, addBaseOptions, addressToScVal } = require('./utils');
const { addOptionsToCommands, getChainConfig } = require('../common');
const { prompt } = require('../common/utils');

async function is_operator(wallet, _, chain, contract, args, options) {
    const [address] = args;
    const operation = contract.call('is_operator', addressToScVal(address));
    const result = await broadcast(operation, wallet, chain, 'is_operator called', options);

    if (result.value()) {
        printInfo(address + ' is an operator');
    } else {
        printWarn(address + ' is not an operator');
    }
}

async function add_operator(wallet, _, chain, contract, args, options) {
    const [address] = args;
    const operation = contract.call('add_operator', addressToScVal(address));
    await broadcast(operation, wallet, chain, 'add_operator called', options);
}

async function remove_operator(wallet, _, chain, contract, args, options) {
    const [address] = args;
    const operation = contract.call('remove_operator', addressToScVal(address));
    await broadcast(operation, wallet, chain, 'remove_operator called', options);
}

async function refund(wallet, _, chain, contract, args, options) {
    const operator = Address.fromString(wallet.publicKey()).toScVal();
    const gasService = chain.contracts?.axelar_gas_service?.address;

    const target = Address.fromString(gasService).toScVal();
    const method = nativeToScVal('refund', { type: 'symbol' });
    const [messageId, receiver, tokenAddress, tokenAmount] = args;

    validateParameters({
        isNonEmptyString: { messageId, receiver, tokenAddress },
        isValidNumber: { tokenAmount },
    });

    const args = nativeToScVal([
        messageId,
        Address.fromString(receiver),
        { address: Address.fromString(tokenAddress), amount: tokenAmount },
    ]);

    const operation = contract.call('execute', operator, target, method, args);

    await broadcast(operation, wallet, chain, 'Executed', options);
}

async function execute(wallet, _, chain, contract, args, options) {
    const operator = Address.fromString(wallet.publicKey()).toScVal();
    const [target, method, params] = args;

    const operation = contract.call(
        'execute',
        operator,
        addressToScVal(target),
        nativeToScVal(method, { type: 'symbol' }),
        nativeToScVal(parseArgs(params)),
    );

    await broadcast(operation, wallet, chain, 'Executed', options);
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

    if (!chain.contracts?.axelar_operators) {
        throw new Error('Operators contract not found.');
    }

    const contract = new Contract(chain.contracts.axelar_operators.address);

    await processor(wallet, config, chain, contract, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('operators').description('Operators contract management');

    program.command('is_operator <address> ').action((address, options) => {
        mainProcessor(is_operator, [address], options);
    });

    program.command('add_operator <address> ').action((address, options) => {
        mainProcessor(add_operator, [address], options);
    });

    program.command('remove_operator <address> ').action((address, options) => {
        mainProcessor(remove_operator, [address], options);
    });

    program
        .command('refund <messageId> <receiver> <tokenAddress> <tokenAmount> ')
        .action((messageId, receiver, tokenAddress, tokenAmount, options) => {
            mainProcessor(refund, [messageId, receiver, tokenAddress, tokenAmount], options);
        });

    program.command('execute <target> <method> <params>').action((target, method, params, options) => {
        mainProcessor(execute, [target, method, params], options);
    });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
