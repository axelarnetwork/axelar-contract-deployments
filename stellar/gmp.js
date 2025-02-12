'use strict';

const { Contract, nativeToScVal } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { getWallet, broadcast, addBaseOptions, tokenToScVal, addressToScVal, hexToScVal } = require('./utils');
const { addOptionsToCommands, getChainConfig } = require('../common');
require('./cli-utils');

async function send(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const caller = addressToScVal(wallet.publicKey());

    const [destinationChain, destinationAddress, payload, gasTokenAddress, gasFeeAmount] = args;

    const operation = contract.call(
        'send',
        caller,
        nativeToScVal(destinationChain, { type: 'string' }),
        nativeToScVal(destinationAddress, { type: 'string' }),
        hexToScVal(payload),
        tokenToScVal(gasTokenAddress, gasFeeAmount),
    );

    await broadcast(operation, wallet, chain, 'Send Called', options);
}

async function execute(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);

    const [sourceChain, messageId, sourceAddress, payload] = args;

    const operation = contract.call(
        'execute',
        nativeToScVal(sourceChain, { type: 'string' }),
        nativeToScVal(messageId, { type: 'string' }),
        nativeToScVal(sourceAddress, { type: 'string' }),
        hexToScVal(payload),
    );

    await broadcast(operation, wallet, chain, 'Execute Called', options);
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);

    printInfo('Environment', options.env);
    printInfo('Chain Name', options.chainName);

    if (!chain.contracts?.example) {
        throw new Error('Example package not found.');
    }

    await processor(wallet, config, chain, chain.contracts.example, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('gmp').description('Example of Stellar gmp commands');

    program
        .command('send <destinationChain> <destinationAddress> <payload> <gasTokenAddress> <gasFeeAmount>')
        .description('Send gmp contract call')
        .action((destinationChain, destinationAddress, payload, gasTokenAddress, gasFeeAmount, options) => {
            mainProcessor(send, [destinationChain, destinationAddress, payload, gasTokenAddress, gasFeeAmount], options);
        });

    program
        .command('execute <sourceChain> <messageId> <sourceAddress> <payload>')
        .description('Execute gmp contract call')
        .action((sourceChain, messageId, sourceAddress, payload, options) => {
            mainProcessor(execute, [sourceChain, messageId, sourceAddress, payload], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
