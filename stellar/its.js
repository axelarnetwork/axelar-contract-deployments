'use strict';

const { Contract, nativeToScVal } = require('@stellar/stellar-sdk');
const { Command } = require('commander');

const { saveConfig, loadConfig, addOptionsToCommands, getChainConfig } = require('../common');
const {
    addBaseOptions,
    getWallet,
    broadcast,
    tokenToScVal,
    tokenMetadataToScVal,
    addressToScVal,
    hexToScVal,
    saltToBytes32,
} = require('./utils');
const { prompt } = require('../common/utils');

async function setTrustedChain(wallet, _, chain, contract, arg, options) {
    const callArg = nativeToScVal(arg, { type: 'string' });

    const operation = contract.call('set_trusted_chain', callArg);

    await broadcast(operation, wallet, chain, 'Trusted Chain Set', options);
}

async function removeTrustedChain(wallet, _, chain, contract, arg, options) {
    const callArg = nativeToScVal(arg, { type: 'string' });

    const operation = contract.call('remove_trusted_chain', callArg);

    await broadcast(operation, wallet, chain, 'Trusted Chain Removed', options);
}

async function deployInterchainToken(wallet, _, chain, contract, args, options) {
    const caller = addressToScVal(wallet.publicKey());
    const minter = caller;
    const [symbol, name, decimal, salt, initialSupply] = args;
    const saltBytes32 = saltToBytes32(salt);

    const operation = contract.call(
        'deploy_interchain_token',
        caller,
        hexToScVal(saltBytes32),
        tokenMetadataToScVal(decimal, name, symbol),
        nativeToScVal(initialSupply, { type: 'i128' }),
        minter,
    );

    await broadcast(operation, wallet, chain, 'Interchain Token Deployed', options);
}

async function deployRemoteInterchainToken(wallet, _, chain, contract, args, options) {
    const caller = addressToScVal(wallet.publicKey());
    const [salt, destinationChain, gasTokenAddress, gasFeeAmount] = args;
    const saltBytes32 = saltToBytes32(salt);

    const operation = contract.call(
        'deploy_remote_interchain_token',
        caller,
        hexToScVal(saltBytes32),
        nativeToScVal(destinationChain, { type: 'string' }),
        tokenToScVal(gasTokenAddress, gasFeeAmount),
    );

    await broadcast(operation, wallet, chain, 'Remote Interchain Token Deployed', options);
}

async function registerCanonicalToken(wallet, _, chain, contract, args, options) {
    const [tokenAddress] = args;

    const operation = contract.call('register_canonical_token', nativeToScVal(tokenAddress, { type: 'address' }));

    await broadcast(operation, wallet, chain, 'Canonical Token Registered', options);
}

async function deployRemoteCanonicalToken(wallet, _, chain, contract, args, options) {
    const spenderScVal = addressToScVal(wallet.publicKey());

    const [tokenAddress, destinationChain, gasTokenAddress, gasFeeAmount] = args;

    const operation = contract.call(
        'deploy_remote_canonical_token',
        nativeToScVal(tokenAddress, { type: 'address' }),
        nativeToScVal(destinationChain, { type: 'string' }),
        spenderScVal,
        tokenToScVal(gasTokenAddress, gasFeeAmount),
    );

    await broadcast(operation, wallet, chain, 'Remote Canonical Token Deployed', options);
}

async function interchainTransfer(wallet, _, chain, contract, args, options) {
    const caller = addressToScVal(wallet.publicKey());
    const [tokenId, destinationChain, destinationAddress, amount, data, gasTokenAddress, gasFeeAmount] = args;

    const operation = contract.call(
        'interchain_transfer',
        caller,
        hexToScVal(tokenId),
        nativeToScVal(destinationChain, { type: 'string' }),
        hexToScVal(destinationAddress),
        nativeToScVal(amount, { type: 'i128' }),
        hexToScVal(data),
        tokenToScVal(gasTokenAddress, gasFeeAmount),
    );

    await broadcast(operation, wallet, chain, 'Interchain Token Transferred', options);
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

    if (!chain.contracts?.interchain_token_service) {
        throw new Error('Interchain Token Service package not found.');
    }

    const contract = new Contract(chain.contracts.interchain_token_service.address);

    await processor(wallet, config, chain, contract, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('its').description('Interchain Token Service contract operations.');

    program
        .command('set-trusted-chain <chainName>')
        .description('set a trusted ITS chain')
        .action((chainName, options) => {
            mainProcessor(setTrustedChain, chainName, options);
        });

    program
        .command('remove-trusted-chain <chainName>')
        .description('remove a trusted ITS chain')
        .action((chainName, options) => {
            mainProcessor(removeTrustedChain, chainName, options);
        });

    program
        .command('deploy-interchain-token <symbol> <name> <decimals> <salt> <initialSupply>')
        .description('deploy interchain token')
        .action((symbol, name, decimal, salt, initialSupply, options) => {
            mainProcessor(deployInterchainToken, [symbol, name, decimal, salt, initialSupply], options);
        });

    program
        .command('deploy-remote-interchain-token <salt> <destinationChain> <gasTokenAddress> <gasFeeAmount>')
        .description('deploy remote interchain token')
        .action((salt, destinationChain, gasTokenAddress, gasFeeAmount, options) => {
            mainProcessor(deployRemoteInterchainToken, [salt, destinationChain, gasTokenAddress, gasFeeAmount], options);
        });

    program
        .command('register-canonical-token <tokenAddress>')
        .description('register canonical token')
        .action((tokenAddress, options) => {
            mainProcessor(registerCanonicalToken, [tokenAddress], options);
        });

    program
        .command('deploy-remote-canonical-token <tokenAddress> <destinationChain> <gasTokenAddress> <gasFeeAmount>')
        .description('deploy remote canonical token')
        .action((tokenAddress, destinationChain, gasTokenAddress, gasFeeAmount, options) => {
            mainProcessor(deployRemoteCanonicalToken, [tokenAddress, destinationChain, gasTokenAddress, gasFeeAmount], options);
        });

    program
        .command('interchain-transfer <tokenId> <destinationChain> <destinationAddress> <amount> <data> <gasTokenAddress> <gasFeeAmount>')
        .description('interchain transfer')
        .action((tokenId, destinationChain, destinationAddress, amount, data, gasTokenAddress, gasFeeAmount, options) => {
            mainProcessor(
                interchainTransfer,
                [tokenId, destinationChain, destinationAddress, amount, data, gasTokenAddress, gasFeeAmount],
                options,
            );
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
