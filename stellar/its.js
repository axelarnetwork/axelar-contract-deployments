'use strict';

const { Address, Contract, nativeToScVal, Operation, xdr, authorizeInvocation, rpc } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, defaultAbiCoder, hexZeroPad, isHexString, keccak256 },
} = ethers;

const { saveConfig, loadConfig, addOptionsToCommands, getChainConfig } = require('../common');
const {
    addBaseOptions,
    getWallet,
    broadcast,
    tokenToScVal,
    tokenMetadataToScVal,
    getNetworkPassphrase,
    createAuthorizedFunc,
} = require('./utils');
const { prompt } = require('../common/utils');

const HUB_CHAIN = 'axelar';

async function setTrustedChain(wallet, _, chain, contractConfig, arg, options) {
    const contract = new Contract(contractConfig.address);
    const callArg = nativeToScVal(arg, { type: 'string' });

    const operation = contract.call('set_trusted_chain', callArg);

    await broadcast(operation, wallet, chain, 'Trusted Chain Set', options);
}

async function removeTrustedChain(wallet, _, chain, arg, options) {
    const contract = new Contract(chain.contracts.interchain_token_service?.address);
    const callArg = nativeToScVal(arg, { type: 'string' });

    const operation = contract.call('remove_trusted_chain', callArg);

    await broadcast(operation, wallet, chain, 'Trusted Chain Removed', options);
}

async function deployInterchainToken(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const caller = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });
    const minter = caller;
    const [symbol, name, decimal, salt, initialSupply] = args;
    const saltBytes32 = isHexString(salt) ? hexZeroPad(salt, 32) : keccak256(salt);

    const operation = contract.call(
        'deploy_interchain_token',
        caller,
        nativeToScVal(Buffer.from(arrayify(saltBytes32)), { type: 'bytes' }),
        tokenMetadataToScVal(decimal, name, symbol),
        nativeToScVal(initialSupply, { type: 'i128' }),
        minter,
    );

    await broadcast(operation, wallet, chain, 'Interchain Token Deployed', options);
}

async function deployRemoteInterchainToken(wallet, _, chain, contractConfig, args, options) {
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

    await broadcast(operation, wallet, chain, 'Remote Interchain Token Deployed', options);
}

async function registerCanonicalToken(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const [tokenAddress] = args;

    const operation = contract.call('register_canonical_token', nativeToScVal(tokenAddress, { type: 'address' }));

    await broadcast(operation, wallet, chain, 'Canonical Token Registered', options);
}

async function createPayGasAndCallContractAuth(spenderScVal, payload, gasTokenAddress, gasFeeAmount, chain, wallet) {
    const { interchain_token_service: { address: itsAddress, initializeArgs: { gasServiceAddress, gatewayAddress, itsHubAddress } } = {} } =
        chain.contracts;

    const itsAddressScVal = nativeToScVal(Address.fromString(itsAddress), { type: 'address' });
    const itsHubChainScVal = nativeToScVal(HUB_CHAIN, { type: 'string' });
    const itsHubAddressScVal = nativeToScVal(itsHubAddress, { type: 'string' });
    const gasServiceAddressScVal = nativeToScVal(gasServiceAddress, { type: 'address' });
    const payloadScVal = nativeToScVal(Buffer.from(arrayify(payload, 'hex')), { type: 'bytes' });
    const gasTokenScVal = tokenToScVal(gasTokenAddress, gasFeeAmount);
    const gasFeeAmountScVal = nativeToScVal(gasFeeAmount, { type: 'i128' });
    const emptyBytesScVal = nativeToScVal(Buffer.from(''), { type: 'bytes' });

    const validUntil = await new rpc.Server(chain.rpc).getLatestLedger().then((info) => info.sequence + 100);

    const transferAuth = new xdr.SorobanAuthorizedInvocation({
        function: createAuthorizedFunc(Address.fromString(gasTokenAddress), 'transfer', [
            spenderScVal,
            gasServiceAddressScVal,
            gasFeeAmountScVal,
        ]),
        subInvocations: [],
    });

    return Promise.all(
        [
            new xdr.SorobanAuthorizedInvocation({
                function: createAuthorizedFunc(Address.fromString(gasServiceAddress), 'pay_gas', [
                    itsAddressScVal,
                    itsHubChainScVal,
                    itsHubAddressScVal,
                    payloadScVal,
                    spenderScVal,
                    gasTokenScVal,
                    emptyBytesScVal,
                ]),
                subInvocations: [transferAuth],
            }),
            new xdr.SorobanAuthorizedInvocation({
                function: createAuthorizedFunc(Address.fromString(gatewayAddress), 'call_contract', [
                    itsAddressScVal,
                    itsHubChainScVal,
                    itsHubAddressScVal,
                    emptyBytesScVal,
                ]),
                subInvocations: [],
            }),
        ].map((auth) => authorizeInvocation(wallet, validUntil, auth, wallet.publicKey(), getNetworkPassphrase(chain.networkType))),
    );
}

async function deployRemoteCanonicalToken(wallet, _, chain, contractConfig, args, options) {
    const itsAddress = contractConfig.address;
    const spenderScVal = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });

    const [tokenAddress, tokenId, tokenName, tokenSymbol, decimals, destinationChain, gasTokenAddress, gasFeeAmount] = args;

    const payload = defaultAbiCoder.encode(
        ['uint8', 'string', 'bytes'],
        [
            3, // HubMessage type for SendToHub
            destinationChain,
            defaultAbiCoder.encode(
                ['uint8', 'bytes32', 'string', 'string', 'uint8', 'bytes'],
                [1, '0x' + tokenId, tokenName, tokenSymbol, decimals, '0x'],
            ),
        ],
    );

    const auth = await createPayGasAndCallContractAuth(spenderScVal, payload, gasTokenAddress, gasFeeAmount, chain, wallet);

    const operation = Operation.invokeContractFunction({
        contract: itsAddress,
        function: 'deploy_remote_canonical_token',
        args: [
            nativeToScVal(tokenAddress, { type: 'address' }),
            nativeToScVal(destinationChain, { type: 'string' }),
            spenderScVal,
            tokenToScVal(gasTokenAddress, gasFeeAmount),
        ],
        auth,
    });

    await broadcast(operation, wallet, chain, 'Remote Canonical Token Deployed', options);
}

async function interchainTransfer(wallet, _, chain, contractConfig, args, options) {
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

    await processor(wallet, config, chain, args, options);

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
        .command('deploy-interchain-token <symbol> <name> <decimals> <salt> <initialSupply> ')
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
        .command(
            'deploy-remote-canonical-token <tokenAddress> <tokenId> <tokenName> <tokenSymbol> <decimals> <destinationChain> <gasTokenAddress> <gasFeeAmount>',
        )
        .description('deploy remote canonical token')
        .action((tokenAddress, tokenId, tokenName, tokenSymbol, decimals, destinationChain, gasTokenAddress, gasFeeAmount, options) => {
            mainProcessor(
                deployRemoteCanonicalToken,
                [tokenAddress, tokenId, tokenName, tokenSymbol, decimals, destinationChain, gasTokenAddress, gasFeeAmount],
                options,
            );
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
