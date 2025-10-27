'use strict';

const { Contract, nativeToScVal } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { getWallet, broadcast, addBaseOptions, tokenToScVal, addressToScVal, hexToScVal } = require('./utils');
const { addOptionsToCommands, getChainConfig, validateParameters } = require('../common');
require('./cli-utils');

async function send(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const caller = addressToScVal(wallet.publicKey());
    const [destinationChain, destinationAddress, payload] = args;

    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;
    const gasAmount = options.gasAmount;

    validateParameters({
        isValidStellarAddress: { gasTokenAddress },
        isValidNumber: { gasAmount },
    });

    const operation = contract.call(
        'send',
        caller,
        nativeToScVal(destinationChain, { type: 'string' }),
        nativeToScVal(destinationAddress, { type: 'string' }),
        hexToScVal(payload),
        tokenToScVal(gasTokenAddress, gasAmount),
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
    const chain = getChainConfig(config.chains, options.chainName);
    const wallet = await getWallet(chain, options);

    printInfo('Environment', options.env);
    printInfo('Chain Name', options.chainName);

    if (!chain.contracts?.AxelarExample) {
        throw new Error('AxelarExample package not found.');
    }

    const contractId = chain.contracts.AxelarExample.address;

    validateParameters({
        isValidStellarAddress: { contractId },
    });

    await processor(wallet, config, chain, chain.contracts.AxelarExample, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('gmp').description('Example of Stellar gmp commands');

    program
        .command('send <destinationChain> <destinationAddress> <payload>')
        .description('Send gmp contract call')
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default(0))
        .action((destinationChain, destinationAddress, payload, options) => {
            mainProcessor(send, [destinationChain, destinationAddress, payload], options);
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
