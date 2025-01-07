'use strict';

const { Contract, Address, nativeToScVal } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const { loadConfig, saveConfig } = require('../evm/utils');
const { getWallet, broadcast, addBaseOptions, tokenToScVal, tokenMetadataToScVal } = require('./utils');
const { addOptionsToCommands, getChainConfig } = require('../common');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, hexZeroPad },
} = ethers;
require('./cli-utils');

async function deployToken(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const caller = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });
    const minter = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });
    const [symbol, name, decimal, salt, initialSupply] = args;
    const saltBytes32 = hexZeroPad(salt.startsWith('0x') ? salt : '0x' + salt, 32);

    const operation = contract.call(
        'deploy_interchain_token',
        caller,
        nativeToScVal(Buffer.from(arrayify(saltBytes32)), { type: 'bytes' }),
        tokenMetadataToScVal(decimal, name, symbol),
        nativeToScVal(initialSupply, { type: 'i128' }),
        minter,
    );

    await broadcast(operation, wallet, chain, 'Deploy Token', options);
}

async function deployRemoteToken(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const caller = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });
    const [salt, destinationChain, gasTokenAddress, gasFeeAmount] = args;
    const saltBytes32 = hexZeroPad(salt.startsWith('0x') ? salt : '0x' + salt, 32);

    const operation = contract.call(
        'deploy_remote_interchain_token',
        caller,
        nativeToScVal(Buffer.from(arrayify(saltBytes32)), { type: 'bytes' }),
        nativeToScVal(destinationChain, { type: 'string' }),
        tokenToScVal(gasTokenAddress, gasFeeAmount),
    );

    await broadcast(operation, wallet, chain, 'Deploy Token', options);
}

async function sendToken(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const caller = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });
    const [tokenId, destinationChain, destinationAddress, amount, data, gasTokenAddress, gasFeeAmount] = args;

    const operation = contract.call(
        'interchain_transfer',
        caller,
        nativeToScVal(Buffer.from(arrayify(tokenId)), { type: 'bytes' }),
        nativeToScVal(destinationChain, { type: 'string' }),
        nativeToScVal(Buffer.from(arrayify(destinationAddress)), { type: 'bytes' }),
        nativeToScVal(amount, { type: 'i128' }),
        nativeToScVal(Buffer.from(arrayify(data)), { type: 'bytes' }),
        tokenToScVal(gasTokenAddress, gasFeeAmount),
    );

    await broadcast(operation, wallet, chain, 'Send Token', options);
}

async function execute(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const [sourceChain, messageId, sourceAddress, payload] = args;

    const operation = contract.call(
        'execute',
        nativeToScVal(sourceChain, { type: 'string' }),
        nativeToScVal(messageId, { type: 'string' }),
        nativeToScVal(sourceAddress, { type: 'string' }),
        nativeToScVal(Buffer.from(arrayify(payload)), { type: 'bytes' }),
    );

    await broadcast(operation, wallet, chain, 'Execute Called', options);
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);

    if (!chain.contracts?.interchain_token_service) {
        throw new Error('Interchain Token Service package not found.');
    }

    await processor(wallet, config, chain, chain.contracts.interchain_token_service, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('its-example').description('Setllar ITS Example scripts.');

    program
        .name('deploy-token')
        .description('deploy interchain token')
        .command('deploy-token <symbol> <name> <decimals> <salt> <initialSupply> ')
        .action((symbol, name, decimal, salt, initialSupply, options) => {
            mainProcessor(deployToken, [symbol, name, decimal, salt, initialSupply], options);
        });

    program
        .name('deploy-remote-token')
        .description('deploy remote interchain token')
        .command('deploy-remote-token <salt> <destinationChain> <gasTokenAddress> <gasFeeAmount>')
        .action((salt, destinationChain, gasTokenAddress, gasFeeAmount, options) => {
            mainProcessor(deployRemoteToken, [salt, destinationChain, gasTokenAddress, gasFeeAmount], options);
        });

    program
        .name('send-token')
        .description('send token')
        .command('send-token <tokenId> <destinationChain> <destinationAddress> <amount> <data> <gasTokenAddress> <gasFeeAmount>')
        .action((tokenId, destinationChain, destinationAddress, amount, data, gasTokenAddress, gasFeeAmount, options) => {
            mainProcessor(sendToken, [tokenId, destinationChain, destinationAddress, amount, data, gasTokenAddress, gasFeeAmount], options);
        });

    program
        .name('execute')
        .description('execute a message')
        .command('execute <sourceChain> <messageId> <sourceAddress> <payload>')
        .action((sourceChain, messageId, sourceAddress, payload, options) => {
            mainProcessor(execute, [sourceChain, messageId, sourceAddress, payload], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
