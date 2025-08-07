'use strict';

const { Contract, nativeToScVal, Operation, Address, authorizeInvocation, rpc, xdr } = require('@stellar/stellar-sdk');
const { Command, Option, Argument } = require('commander');
const {
    saveConfig,
    loadConfig,
    addOptionsToCommands,
    getChainConfig,
    getChainConfigByAxelarId,
    printInfo,
    printWarn,
    printError,
    validateParameters,
    validateDestinationChain,
} = require('../common');
const {
    addBaseOptions,
    getWallet,
    broadcast,
    tokenToScVal,
    tokenMetadataToScVal,
    addressToScVal,
    hexToScVal,
    saltToBytes32,
    serializeValue,
    createAuthorizedFunc,
    getNetworkPassphrase,
    getAuthValidUntilLedger,
} = require('./utils');
const { prompt, parseTrustedChains, encodeITSDestination, tokenManagerTypes, validateLinkType } = require('../common/utils');

async function manageTrustedChains(action, wallet, config, chain, contract, args, options) {
    const trustedChains = parseTrustedChains(config, args);

    for (const trustedChain of trustedChains) {
        printInfo(action, trustedChain);

        const trustedChainScVal = nativeToScVal(trustedChain, { type: 'string' });

        try {
            const isTrusted = (
                await broadcast(contract.call('is_trusted_chain', trustedChainScVal), wallet, chain, 'Is trusted chain', options)
            ).value();

            if (isTrusted && action === 'set_trusted_chain') {
                printWarn('The chain is already trusted', trustedChain);
                continue;
            }

            if (!isTrusted && action === 'remove_trusted_chain') {
                printWarn('The chain is not trusted', trustedChain);
                continue;
            }

            await broadcast(contract.call(action, trustedChainScVal), wallet, chain, action, options);
            printInfo(`Successfully ${action === 'set_trusted_chain' ? 'added' : 'removed'} trusted chain`, trustedChain);
        } catch (error) {
            printError(`Failed to process ${action}`, trustedChain, error);
        }
    }
}

async function addTrustedChains(wallet, config, chain, contract, args, options) {
    await manageTrustedChains('set_trusted_chain', wallet, config, chain, contract, args, options);
}

async function removeTrustedChains(wallet, config, chain, contract, args, options) {
    await manageTrustedChains('remove_trusted_chain', wallet, config, chain, contract, args, options);
}

async function deployInterchainToken(wallet, _config, chain, contract, args, options) {
    const caller = addressToScVal(wallet.publicKey());
    const minter = caller;
    const [symbol, name, decimal, salt, initialSupply] = args;
    const saltBytes32 = saltToBytes32(salt);

    validateParameters({
        isNonEmptyString: { symbol, name },
        isValidNumber: { decimal, initialSupply },
    });

    printInfo('Salt', salt);
    printInfo('Deployment salt (bytes32)', saltBytes32);

    const operation = contract.call(
        'deploy_interchain_token',
        caller,
        hexToScVal(saltBytes32),
        tokenMetadataToScVal(decimal, name, symbol),
        nativeToScVal(initialSupply, { type: 'i128' }),
        minter,
    );

    const returnValue = await broadcast(operation, wallet, chain, 'Interchain Token Deployed', options);
    printInfo('tokenId', serializeValue(returnValue.value()));
}

async function deployRemoteInterchainToken(wallet, _config, chain, contract, args, options) {
    const caller = addressToScVal(wallet.publicKey());
    const [salt, destinationChain] = args;
    const saltBytes32 = saltToBytes32(salt);
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;
    const gasAmount = options.gasAmount;

    validateParameters({
        isValidStellarAddress: { gasTokenAddress },
        isValidNumber: { gasAmount },
    });

    printInfo('Salt', salt);
    printInfo('Deployment salt (bytes32)', saltBytes32);

    const operation = contract.call(
        'deploy_remote_interchain_token',
        caller,
        hexToScVal(saltBytes32),
        nativeToScVal(destinationChain, { type: 'string' }),
        tokenToScVal(gasTokenAddress, gasAmount),
    );

    const returnValue = await broadcast(operation, wallet, chain, 'Remote Interchain Token Deployed', options);
    printInfo('tokenId', serializeValue(returnValue.value()));
}

async function registerCanonicalToken(wallet, _config, chain, contract, args, options) {
    const [tokenAddress] = args;

    const operation = contract.call('register_canonical_token', nativeToScVal(tokenAddress, { type: 'address' }));

    const returnValue = await broadcast(operation, wallet, chain, 'Canonical Token Registered', options);
    printInfo('tokenId', serializeValue(returnValue.value()));
}

async function deployRemoteCanonicalToken(wallet, _config, chain, contract, args, options) {
    const spenderScVal = addressToScVal(wallet.publicKey());
    const [tokenAddress, destinationChain] = args;
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;
    const gasAmount = options.gasAmount;

    validateParameters({
        isValidStellarAddress: { gasTokenAddress },
        isValidNumber: { gasAmount },
    });

    const operation = contract.call(
        'deploy_remote_canonical_token',
        nativeToScVal(tokenAddress, { type: 'address' }),
        nativeToScVal(destinationChain, { type: 'string' }),
        spenderScVal,
        tokenToScVal(gasTokenAddress, gasAmount),
    );

    const returnValue = await broadcast(operation, wallet, chain, 'Remote Canonical Token Deployed', options);
    printInfo('tokenId', serializeValue(returnValue.value()));
}

async function interchainTransfer(wallet, config, chain, contract, args, options) {
    const caller = addressToScVal(wallet.publicKey());
    const [tokenId, destinationChain, destinationAddress, amount] = args;
    const data = options.data === '' ? nativeToScVal(null, { type: 'null' }) : hexToScVal(options.data);
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;
    const gasAmount = options.gasAmount;

    validateParameters({
        isValidStellarAddress: { gasTokenAddress },
        isValidNumber: { gasAmount },
    });

    const itsDestinationAddress = encodeITSDestination(config, destinationChain, destinationAddress);
    printInfo('Human-readable destination address', destinationAddress);

    const operation = contract.call(
        'interchain_transfer',
        caller,
        hexToScVal(tokenId),
        nativeToScVal(destinationChain, { type: 'string' }),
        hexToScVal(itsDestinationAddress),
        nativeToScVal(amount, { type: 'i128' }),
        data,
        tokenToScVal(gasTokenAddress, gasAmount),
    );

    await broadcast(operation, wallet, chain, 'Interchain Token Transferred', options);
}

async function execute(wallet, _config, chain, contract, args, options) {
    const [sourceChain, messageId, sourceAddress, payload] = args;

    const operation = contract.call(
        'execute',
        nativeToScVal(sourceChain, { type: 'string' }),
        nativeToScVal(messageId, { type: 'string' }),
        nativeToScVal(sourceAddress, { type: 'string' }),
        hexToScVal(payload),
    );

    await broadcast(operation, wallet, chain, 'Executed', options);
}

async function flowLimit(wallet, _config, chain, contract, args, options) {
    const [tokenId] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
    });

    const operation = contract.call('flow_limit', hexToScVal(tokenId));

    const returnValue = await broadcast(operation, wallet, chain, 'Get Flow Limit', options);
    const flowLimit = returnValue.value();

    printInfo('Flow Limit', flowLimit || 'No limit set');
}

async function setFlowLimit(wallet, _config, chain, contract, args, options) {
    const [tokenId, flowLimit] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
        isValidNumber: { flowLimit },
    });

    const flowLimitScVal = nativeToScVal(flowLimit, { type: 'i128' });

    const operation = contract.call('set_flow_limit', hexToScVal(tokenId), flowLimitScVal);

    await broadcast(operation, wallet, chain, 'Set Flow Limit', options);
    printInfo('Successfully set flow limit', flowLimit);
}

async function removeFlowLimit(wallet, _config, chain, contract, args, options) {
    const [tokenId] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
    });

    const flowLimitScVal = nativeToScVal(null, { type: 'void' });

    const operation = contract.call('set_flow_limit', hexToScVal(tokenId), flowLimitScVal);

    await broadcast(operation, wallet, chain, 'Remove Flow Limit', options);
    printInfo('Successfully removed flow limit');
}

async function interchainTokenAddress(wallet, _config, chain, contract, args, options) {
    const [tokenId] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
    });

    const tokenIdBytes = hexToScVal(tokenId);
    const operation = contract.call('interchain_token_address', tokenIdBytes);

    const returnValue = await broadcast(operation, wallet, chain, 'Get interchain token address', options);
    const tokenAddress = serializeValue(returnValue.value());

    printInfo(`Interchain Token Address`, tokenAddress);

    return tokenAddress;
}

async function registeredTokenAddress(wallet, _config, chain, contract, args, options) {
    const [tokenId] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
    });

    const tokenIdBytes = hexToScVal(tokenId);
    const operation = contract.call('registered_token_address', tokenIdBytes);

    const returnValue = await broadcast(operation, wallet, chain, 'Get registered token address', options);
    const tokenAddress = serializeValue(returnValue.value());

    printInfo(`Registered Token Address`, tokenAddress);

    return tokenAddress;
}

async function tokenAdmin(wallet, _config, chain, contract, args, options) {
    const [tokenId] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
    });

    const tokenIdBytes = hexToScVal(tokenId);
    const operation = contract.call('token_admin', tokenIdBytes);

    const returnValue = await broadcast(operation, wallet, chain, 'Get token admin', options);
    const adminAddress = serializeValue(returnValue.value());

    printInfo(`Token Admin Address`, adminAddress);

    return adminAddress;
}

async function deployedTokenManager(wallet, _config, chain, contract, args, options) {
    const [tokenId] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
    });

    const tokenIdBytes = hexToScVal(tokenId);

    const operation = contract.call('deployed_token_manager', tokenIdBytes);

    const returnValue = await broadcast(operation, wallet, chain, 'Get deployed token manager', options);
    const tokenManagerAddress = serializeValue(returnValue.value());

    printInfo(`Deployed Token Manager Address`, tokenManagerAddress);

    return tokenManagerAddress;
}

async function registerTokenMetadata(wallet, _config, chain, contract, args, options) {
    const [tokenAddress] = args;
    const spender = addressToScVal(wallet.publicKey());
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;
    const gasAmount = options.gasAmount;

    validateParameters({
        isValidStellarAddress: { tokenAddress, gasTokenAddress },
        isValidNumber: { gasAmount },
    });

    const operation = contract.call(
        'register_token_metadata',
        nativeToScVal(tokenAddress, { type: 'address' }),
        spender,
        tokenToScVal(gasTokenAddress, gasAmount),
    );

    await broadcast(operation, wallet, chain, 'Token Metadata Registered', options);
}

async function registerCustomToken(wallet, _config, chain, contract, args, options) {
    const deployer = addressToScVal(wallet.publicKey());
    const [salt, tokenAddress, type] = args;
    const saltBytes32 = saltToBytes32(salt);

    validateParameters({
        isValidStellarAddress: { tokenAddress },
        isNonEmptyString: { type },
    });

    const tokenManagerType = validateLinkType(chain.chainType, type);

    printInfo('Salt', salt);
    printInfo('Deployment salt (bytes32)', saltBytes32);

    const operation = contract.call(
        'register_custom_token',
        deployer,
        hexToScVal(saltBytes32),
        nativeToScVal(tokenAddress, { type: 'address' }),
        nativeToScVal(tokenManagerType, { type: 'u32' }),
    );

    const returnValue = await broadcast(operation, wallet, chain, 'Custom Token Registered', options);
    printInfo('tokenId', serializeValue(returnValue.value()));
}

async function linkToken(wallet, config, chain, contract, args, options) {
    const deployer = addressToScVal(wallet.publicKey());
    const [salt, destinationChain, destinationTokenAddress, type] = args;
    const saltBytes32 = saltToBytes32(salt);
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;
    const gasAmount = options.gasAmount;

    validateParameters({
        isValidStellarAddress: { gasTokenAddress },
        isValidNumber: { gasAmount },
        isNonEmptyString: { destinationChain, destinationTokenAddress, type },
    });
    validateDestinationChain(config, destinationChain);

    const tokenManagerType = validateLinkType(getChainConfigByAxelarId(config, destinationChain).chainType, type);

    printInfo('Salt', salt);
    printInfo('Deployment salt (bytes32)', saltBytes32);

    const itsDestinationTokenAddress = encodeITSDestination(config, destinationChain, destinationTokenAddress);
    printInfo('Human-readable destination token address', destinationTokenAddress);

    let operatorBytes = nativeToScVal(null, { type: 'void' });
    if (options.operator) {
        printInfo('Destination Operator address', options.operator);
        operatorBytes = hexToScVal(encodeITSDestination(config, destinationChain, options.operator));
    }

    const operation = contract.call(
        'link_token',
        deployer,
        hexToScVal(saltBytes32),
        nativeToScVal(destinationChain, { type: 'string' }),
        hexToScVal(itsDestinationTokenAddress),
        nativeToScVal(tokenManagerType, { type: 'u32' }),
        operatorBytes,
        tokenToScVal(gasTokenAddress, gasAmount),
    );

    const returnValue = await broadcast(operation, wallet, chain, 'Token Linked', options);
    printInfo('tokenId', serializeValue(returnValue.value()));
}

async function createMintAuths(tokenAddress, tokenAddressScVal, tokenManager, recipientScVal, amountScVal, wallet, chain) {
    const publicKey = wallet.publicKey();
    const networkPassphrase = getNetworkPassphrase(chain.networkType);

    const validUntil = await getAuthValidUntilLedger(chain);

    const mintAuth = createAuthorizedFunc(Address.fromString(tokenAddress), 'mint', [recipientScVal, amountScVal]);
    const executeAuth = createAuthorizedFunc(Address.fromString(tokenManager), 'execute', [
        tokenAddressScVal,
        nativeToScVal('mint', { type: 'symbol' }),
        nativeToScVal([recipientScVal, amountScVal]),
    ]);

    const [mintInvocation, executeInvocation] = [
        new xdr.SorobanAuthorizedInvocation({ function: mintAuth, subInvocations: [] }),
        new xdr.SorobanAuthorizedInvocation({ function: executeAuth, subInvocations: [] }),
    ];

    return Promise.all([
        authorizeInvocation(wallet, validUntil, mintInvocation, publicKey, networkPassphrase),
        authorizeInvocation(wallet, validUntil, executeInvocation, publicKey, networkPassphrase),
    ]);
}

async function testExecute(wallet, _config, chain, contract, args, options) {
    const [recipient, tokenAddress, tokenManager, amount] = args;

    validateParameters({
        isValidStellarAddress: { recipient, tokenAddress, tokenManager },
        isValidNumber: { amount },
    });

    const recipientScVal = nativeToScVal(recipient, { type: 'address' });
    const tokenAddressScVal = nativeToScVal(tokenAddress, { type: 'address' });
    const tokenManagerScVal = nativeToScVal(tokenManager, { type: 'address' });
    const amountScVal = nativeToScVal(amount, { type: 'i128' });

    let operation = Operation.invokeContractFunction({
        contract: contract.contractId(),
        function: 'test_execute',
        args: [recipientScVal, tokenAddressScVal, tokenManagerScVal, amountScVal],
        auth: await createMintAuths(tokenAddress, tokenAddressScVal, tokenManager, recipientScVal, amountScVal, wallet, chain),
    });

    await broadcast(operation, wallet, chain, 'Test Execute', options);
    printInfo('Successfully executed test for recipient', recipient);
}

async function testExecute22(wallet, _config, chain, contract, args, options) {
    const [recipient, tokenAddress, tokenManager, amount] = args;

    validateParameters({
        isValidStellarAddress: { recipient, tokenAddress, tokenManager },
        isValidNumber: { amount },
    });

    const recipientScVal = nativeToScVal(recipient, { type: 'address' });
    const tokenAddressScVal = nativeToScVal(tokenAddress, { type: 'address' });
    const tokenManagerScVal = nativeToScVal(tokenManager, { type: 'address' });
    const amountScVal = nativeToScVal(amount, { type: 'i128' });

    const operation = contract.call('test_execute', recipientScVal, tokenAddressScVal, tokenManagerScVal, amountScVal);

    await broadcast(operation, wallet, chain, 'Test Execute', options);
    printInfo('Successfully executed test for recipient', recipient);
}

async function addMinter(wallet, _, chain, contract, args, options) {
    const [tokenAddress, minter] = args;

    validateParameters({
        isValidStellarAddress: { tokenAddress, minter },
    });

    const operation = contract.call(
        'add_minter',
        nativeToScVal(tokenAddress, { type: 'address' }),
        nativeToScVal(minter, { type: 'address' }),
    );

    await broadcast(operation, wallet, chain, 'Add Minter', options);
    printInfo('Successfully added minter for token', tokenAddress);
    printInfo('Minter address', minter);
}

async function transferTokenAdmin(wallet, _config, chain, contract, args, options) {
    const [tokenId, newAdmin] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
        isValidStellarAddress: { newAdmin },
    });

    const tokenIdBytes = hexToScVal(tokenId);
    const deployed_token_manager_operation = contract.call('deployed_token_manager', tokenIdBytes);

    const returnValue1 = await broadcast(deployed_token_manager_operation, wallet, chain, 'Get deployed token manager', options);
    const tokenManagerAddress = serializeValue(returnValue1.value());

    printInfo(`Deployed Token Manager Address`, tokenManagerAddress);

    const registered_token_address_operation = contract.call('registered_token_address', tokenIdBytes);

    const returnValue2 = await broadcast(registered_token_address_operation, wallet, chain, 'Get registered token address', options);
    const registeredTokenAddress = serializeValue(returnValue2.value());

    printInfo(`Registered Token Address`, registeredTokenAddress);

    const tokenManagerContract = new Contract(tokenManagerAddress);
    const operation = tokenManagerContract.call(
        'transfer_admin',
        nativeToScVal(wallet.publicKey(), { type: 'address' }),
        nativeToScVal(registeredTokenAddress, { type: 'address' }),
        nativeToScVal(newAdmin, { type: 'address' }),
    );

    await broadcast(operation, wallet, chain, 'Transfer Token Admin', options);
    printInfo('Successfully transferred admin for token', registeredTokenAddress);
    printInfo('New admin address', newAdmin);
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

    if (!chain.contracts?.InterchainTokenService) {
        throw new Error('Interchain Token Service package not found.');
    }

    const contractId = chain.contracts.InterchainTokenService.address;

    validateParameters({
        isValidStellarAddress: { contractId },
    });

    const contract = new Contract(chain.contracts.InterchainTokenService.address);

    await processor(wallet, config, chain, contract, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('its').description('Interchain Token Service contract operations.');

    program
        .command('add-trusted-chains <trusted-chains...>')
        .description(`Add trusted chains. The <trusted-chains> can be a list of chains separated by whitespaces or special tag 'all'`)
        .action((trustedChains, options) => {
            mainProcessor(addTrustedChains, trustedChains, options);
        });

    program
        .command('remove-trusted-chains <trusted-chains...>')
        .description(`Remove trusted chains. The <trusted-chains> can be a list of chains separated by whitespaces or special tag 'all'`)
        .action((trustedChains, options) => {
            mainProcessor(removeTrustedChains, trustedChains, options);
        });

    program
        .command('deploy-interchain-token <symbol> <name> <decimals> <salt> <initialSupply>')
        .description('deploy interchain token')
        .action((symbol, name, decimal, salt, initialSupply, options) => {
            mainProcessor(deployInterchainToken, [symbol, name, decimal, salt, initialSupply], options);
        });

    program
        .command('deploy-remote-interchain-token <salt> <destinationChain>')
        .description('deploy remote interchain token')
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default(0))
        .action((salt, destinationChain, options) => {
            mainProcessor(deployRemoteInterchainToken, [salt, destinationChain], options);
        });

    program
        .command('register-canonical-token <tokenAddress>')
        .description('register canonical token')
        .action((tokenAddress, options) => {
            mainProcessor(registerCanonicalToken, [tokenAddress], options);
        });

    program
        .command('deploy-remote-canonical-token <tokenAddress> <destinationChain>')
        .description('deploy remote canonical token')
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default(0))
        .action((tokenAddress, destinationChain, options) => {
            mainProcessor(deployRemoteCanonicalToken, [tokenAddress, destinationChain], options);
        });

    program
        .command('register-token-metadata <tokenAddress>')
        .description('register token metadata')
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default(0))
        .action((tokenAddress, options) => {
            mainProcessor(registerTokenMetadata, [tokenAddress], options);
        });

    program
        .command('register-custom-token <salt> <tokenAddress>')
        .description('register custom token')
        .addArgument(new Argument('<type>', 'token manager type').choices(Object.keys(tokenManagerTypes)))
        .action((salt, tokenAddress, type, options) => {
            mainProcessor(registerCustomToken, [salt, tokenAddress, type], options);
        });

    program
        .command('link-token <salt> <destinationChain> <destinationTokenAddress>')
        .description('link token')
        .addArgument(new Argument('<type>', 'token manager type').choices(Object.keys(tokenManagerTypes)))
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default(0))
        .addOption(new Option('--operator <operator>', 'operator for the token id on the destination chain'))
        .action((salt, destinationChain, destinationTokenAddress, type, options) => {
            mainProcessor(linkToken, [salt, destinationChain, destinationTokenAddress, type], options);
        });

    program
        .command('interchain-transfer <tokenId> <destinationChain> <destinationAddress> <amount>')
        .description('interchain transfer')
        .addOption(new Option('--data <data>', 'data').default(''))
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default(0))
        .action((tokenId, destinationChain, destinationAddress, amount, options) => {
            mainProcessor(interchainTransfer, [tokenId, destinationChain, destinationAddress, amount], options);
        });

    program
        .command('execute <sourceChain> <messageId> <sourceAddress> <payload>')
        .description('Execute ITS message')
        .action((sourceChain, messageId, sourceAddress, payload, options) => {
            mainProcessor(execute, [sourceChain, messageId, sourceAddress, payload], options);
        });

    program
        .command('flow-limit <tokenId>')
        .description('Get the flow limit for a token')
        .action((tokenId, options) => {
            mainProcessor(flowLimit, [tokenId], options);
        });

    program
        .command('set-flow-limit <tokenId> <flowLimit>')
        .description('Set the flow limit for a token')
        .action((tokenId, flowLimit, options) => {
            mainProcessor(setFlowLimit, [tokenId, flowLimit], options);
        });

    program
        .command('remove-flow-limit <tokenId>')
        .description('Remove the flow limit for a token')
        .action((tokenId, options) => {
            mainProcessor(removeFlowLimit, [tokenId], options);
        });

    program
        .command('interchain-token-address <tokenId>')
        .description('Get the interchain token address with the given token id')
        .action((tokenId, options) => {
            mainProcessor(interchainTokenAddress, [tokenId], options);
        });

    program
        .command('registered-token-address <tokenId>')
        .description('Get the registered token address with the given token id')
        .action((tokenId, options) => {
            mainProcessor(registeredTokenAddress, [tokenId], options);
        });

    program
        .command('token-admin <tokenId>')
        .description('Get the admin address for a token with the given token id')
        .action((tokenId, options) => {
            mainProcessor(tokenAdmin, [tokenId], options);
        });

    program
        .command('deployed-token-manager <tokenId>')
        .description('Get the deployed token manager address with the given token id')
        .action((tokenId, options) => {
            mainProcessor(deployedTokenManager, [tokenId], options);
        });

    program
        .command('test-execute <recipient> <tokenAddress> <tokenManager> <amount>')
        .description('test execute')
        .action((recipient, tokenAddress, tokenManager, amount, options) => {
            mainProcessor(testExecute, [recipient, tokenAddress, tokenManager, amount], options);
        });

    program
        .command('add-minter <tokenAddress> <minter>')
        .description('Add a minter to a token contract via ITS (only operator)')
        .action((tokenAddress, minter, options) => {
            mainProcessor(addMinter, [tokenAddress, minter], options);
        });

    program
        .command('transfer-token-admin <tokenId> <newAdmin>')
        .description('Transfer admin of a token contract from token id')
        .action((tokenId, newAdmin, options) => {
            mainProcessor(transferTokenAdmin, [tokenId, newAdmin], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
