'use strict';

const { Contract, nativeToScVal } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const { loadConfig, printInfo, printWarn, parseArgs, validateParameters, saveConfig } = require('../evm/utils');
const { getWallet, broadcast, addBaseOptions, addressToScVal, tokenToScVal } = require('./utils');
const { addOptionsToCommands, getChainConfig } = require('../common');
const { prompt } = require('../common/utils');

async function isOperator(wallet, _, chain, contract, args, options) {
    const [address] = args;
    const operation = contract.call('is_operator', addressToScVal(address));
    const result = await broadcast(operation, wallet, chain, 'is_operator called', options);

    if (result.value()) {
        printInfo(address + ' is an operator');
    } else {
        printWarn(address + ' is not an operator');
    }
}

async function addOperator(wallet, _, chain, contract, args, options) {
    const [address] = args;
    const operation = contract.call('add_operator', addressToScVal(address));
    await broadcast(operation, wallet, chain, 'add_operator called', options);
}

async function removeOperator(wallet, _, chain, contract, args, options) {
    const [address] = args;
    const operation = contract.call('remove_operator', addressToScVal(address));
    await broadcast(operation, wallet, chain, 'remove_operator called', options);
}

async function addGas(wallet, _, chain, contract, args, options) {
    const operator = addressToScVal(wallet.publicKey());
    const [sender, messageId, spender, tokenAddress, tokenAmount] = args;

    validateParameters({
        isNonEmptyString: { sender, messageId, spender, tokenAddress },
        isValidNumber: { tokenAmount },
    });

    const target = addressToScVal(chain.contracts?.axelar_gas_service?.address);
    const method = nativeToScVal('add_gas', { type: 'symbol' });
    const params = nativeToScVal([
        addressToScVal(sender),
        nativeToScVal(messageId, { type: 'string' }),
        addressToScVal(spender),
        tokenToScVal(tokenAddress, tokenAmount),
    ]);

    const operation = contract.call('execute', operator, target, method, params);

    await broadcast(operation, wallet, chain, 'add_gas called', options);
}

async function collectFees(wallet, _, chain, contract, args, options) {
    const operator = addressToScVal(wallet.publicKey());
    const [receiver, tokenAddress, tokenAmount] = args;

    validateParameters({
        isNonEmptyString: { receiver, tokenAddress },
        isValidNumber: { tokenAmount },
    });

    const target = addressToScVal(chain.contracts?.axelar_gas_service?.address);
    const method = nativeToScVal('collect_fees', { type: 'symbol' });
    const params = nativeToScVal([addressToScVal(receiver), tokenToScVal(tokenAddress, tokenAmount)]);

    const operation = contract.call('execute', operator, target, method, params);

    await broadcast(operation, wallet, chain, 'collect_fees called', options);
}

async function refund(wallet, _, chain, contract, args, options) {
    const operator = addressToScVal(wallet.publicKey());
    const [messageId, receiver, tokenAddress, tokenAmount] = args;

    validateParameters({
        isNonEmptyString: { messageId, receiver, tokenAddress },
        isValidNumber: { tokenAmount },
    });

    const target = addressToScVal(chain.contracts?.axelar_gas_service?.address);
    const method = nativeToScVal('refund', { type: 'symbol' });
    const params = nativeToScVal([
        nativeToScVal(messageId, { type: 'string' }),
        addressToScVal(receiver),
        tokenToScVal(tokenAddress, tokenAmount),
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
        mainProcessor(isOperator, [address], options);
    });

    program.command('add_operator <address> ').action((address, options) => {
        mainProcessor(addOperator, [address], options);
    });

    program.command('remove_operator <address> ').action((address, options) => {
        mainProcessor(removeOperator, [address], options);
    });

    program
        .command('add_gas <sender> <messageId> <spender> <tokenAddress> <tokenAmount> ')
        .action((sender, messageId, spender, tokenAddress, tokenAmount, options) => {
            mainProcessor(addGas, [sender, messageId, spender, tokenAddress, tokenAmount], options);
        });

    program.command('collect_fees <receiver> <tokenAddress> <tokenAmount> ').action((receiver, tokenAddress, tokenAmount, options) => {
        mainProcessor(collectFees, [receiver, tokenAddress, tokenAmount], options);
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
