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
    validateChain,
    encodeITSDestinationToken,
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
const {
    prompt,
    parseTrustedChains,
    encodeITSDestination,
    tokenManagerTypes,
    validateLinkType,
    estimateITSFee,
} = require('../common/utils');

async function tokenMetadataFromId(wallet, chain, itsContract, tokenId, options) {
    const tokenAddressResult = await broadcast(
        itsContract.call('registered_token_address', hexToScVal(tokenId)),
        wallet,
        chain,
        'Get registered token address',
        options,
    );
    const tokenAddress = serializeValue(tokenAddressResult.value());
    const tokenContract = new Contract(tokenAddress);

    const symbolResult = await broadcast(tokenContract.call('symbol'), wallet, chain, 'Get token symbol', options);
    const decimalsResult = await broadcast(tokenContract.call('decimals'), wallet, chain, 'Get token decimals', options);

    const symbol = serializeValue(symbolResult.value());
    const decimals = decimalsResult.value();

    return { tokenAddress, symbol, decimals };
}

function formatTokenAmount(amount, decimals) {
    const str = amount.toString();

    if (decimals === 0) {return str;}

    const padded = str.padStart(decimals + 1, '0');
    const intPart = padded.slice(0, padded.length - decimals);
    const fracPart = padded.slice(padded.length - decimals).replace(/0+$/, '');

    return fracPart ? `${intPart}.${fracPart}` : intPart;
}

function parseTokenAmount(amount, decimals) {
    const [intPart = '0', fracPart = ''] = amount.split('.');

    if (fracPart.length > decimals) {
        throw new Error(`Too many decimal places for amount ${amount} (max ${decimals})`);
    }

    const raw = intPart + fracPart.padEnd(decimals, '0');
    return raw.replace(/^0+/, '') || '0';
}

async function manageTrustedChains(action, wallet, config, chain, contract, args, options) {
    const trustedChains = parseTrustedChains(config.chains, args);

    for (const trustedChain of trustedChains) {
        printInfo(action, trustedChain);

        try {
            const trustedChainScVal = nativeToScVal(trustedChain, { type: 'string' });
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
            throw new Error(error);
        }
    }
}

async function addTrustedChains(wallet, config, chain, contract, args, options) {
    await manageTrustedChains('set_trusted_chain', wallet, config, chain, contract, args, options);
}

async function removeTrustedChains(wallet, config, chain, contract, args, options) {
    await manageTrustedChains('remove_trusted_chain', wallet, config, chain, contract, args, options);
}

async function isTrustedChain(wallet, _config, chain, contract, args, options) {
    const [trustedChain] = args;

    validateParameters({
        isNonEmptyString: { trustedChain },
    });

    try {
        const trustedChainScVal = nativeToScVal(trustedChain, { type: 'string' });
        const isTrusted = (
            await broadcast(contract.call('is_trusted_chain', trustedChainScVal), wallet, chain, 'Is trusted chain', options)
        ).value();

        if (isTrusted) {
            printInfo(`${trustedChain} is a trusted chain`);
        } else {
            printInfo(`${trustedChain} is not a trusted chain`);
        }
    } catch (error) {
        printError(`Failed to check trusted chain`, trustedChain, error);
        throw new Error(error);
    }
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

    const response = await broadcast(operation, wallet, chain, 'Interchain Token Deployed', options);
    printInfo('tokenId', serializeValue(response.value()));
}

async function deployRemoteInterchainToken(wallet, config, chain, contract, args, options) {
    const caller = addressToScVal(wallet.publicKey());
    const [salt, destinationChain] = args;
    const saltBytes32 = saltToBytes32(salt);
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;

    validateParameters({
        isValidStellarAddress: { gasTokenAddress },
    });

    const { gasFeeValue } = await estimateITSFee(
        chain,
        destinationChain,
        options.env,
        'InterchainTokenDeploymentStarted',
        options.gasAmount,
        config.axelar,
    );

    printInfo('Salt', salt);
    printInfo('Deployment salt (bytes32)', saltBytes32);
    printInfo('Gas fee value', gasFeeValue);

    const operation = contract.call(
        'deploy_remote_interchain_token',
        caller,
        hexToScVal(saltBytes32),
        nativeToScVal(destinationChain, { type: 'string' }),
        tokenToScVal(gasTokenAddress, gasFeeValue),
    );

    const response = await broadcast(operation, wallet, chain, 'Remote Interchain Token Deployed', options);
    printInfo('tokenId', serializeValue(response.value()));
}

async function registerCanonicalToken(wallet, _config, chain, contract, args, options) {
    const [tokenAddress] = args;

    const operation = contract.call('register_canonical_token', nativeToScVal(tokenAddress, { type: 'address' }));

    const response = await broadcast(operation, wallet, chain, 'Canonical Token Registered', options);
    printInfo('tokenId', serializeValue(response.value()));
}

async function deployRemoteCanonicalToken(wallet, config, chain, contract, args, options) {
    const spenderScVal = addressToScVal(wallet.publicKey());
    const [tokenAddress, destinationChain] = args;
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;

    validateParameters({
        isValidStellarAddress: { gasTokenAddress },
    });

    const { gasFeeValue } = await estimateITSFee(
        chain,
        destinationChain,
        options.env,
        'InterchainTokenDeploymentStarted',
        options.gasAmount,
        config.axelar,
    );

    printInfo('Gas fee value', gasFeeValue);

    const operation = contract.call(
        'deploy_remote_canonical_token',
        nativeToScVal(tokenAddress, { type: 'address' }),
        nativeToScVal(destinationChain, { type: 'string' }),
        spenderScVal,
        tokenToScVal(gasTokenAddress, gasFeeValue),
    );

    const response = await broadcast(operation, wallet, chain, 'Remote Canonical Token Deployed', options);
    printInfo('tokenId', serializeValue(response.value()));
}

async function interchainTransfer(wallet, config, chain, contract, args, options) {
    const caller = addressToScVal(wallet.publicKey());
    const [tokenId, destinationChain, destinationAddress, amount] = args;
    const data = options.data === '' ? nativeToScVal(null, { type: 'null' }) : hexToScVal(options.data);
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;

    validateParameters({
        isValidStellarAddress: { gasTokenAddress },
    });

    const { symbol, decimals } = await tokenMetadataFromId(wallet, chain, contract, tokenId, options);
    const amountInUnits = parseTokenAmount(amount, decimals);

    const { gasFeeValue } = await estimateITSFee(
        chain,
        destinationChain,
        options.env,
        'InterchainTransfer',
        options.gasAmount,
        config.axelar,
    );

    const itsDestinationAddress = encodeITSDestination(config.chains, destinationChain, destinationAddress);
    printInfo('Human-readable destination address', destinationAddress);
    printInfo('Transfer amount', `${amount} ${symbol}`);
    printInfo('Gas fee value', gasFeeValue);

    const operation = contract.call(
        'interchain_transfer',
        caller,
        hexToScVal(tokenId),
        nativeToScVal(destinationChain, { type: 'string' }),
        hexToScVal(itsDestinationAddress),
        nativeToScVal(amountInUnits, { type: 'i128' }),
        data,
        tokenToScVal(gasTokenAddress, gasFeeValue),
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
    const response = await broadcast(operation, wallet, chain, 'Get Flow Limit', options);
    const flowLimitValue = response.value();

    if (flowLimitValue === undefined || flowLimitValue === null) {
        printInfo('Flow Limit', 'No limit set');
        return;
    }

    const { symbol, decimals } = await tokenMetadataFromId(wallet, chain, contract, tokenId, options);
    printInfo('Flow Limit', `${formatTokenAmount(flowLimitValue, decimals)} ${symbol}`);
}

async function setFlowLimit(wallet, _config, chain, contract, args, options) {
    const [tokenId, flowLimit] = args;

    validateParameters({
        isNonEmptyString: { tokenId, flowLimit },
    });

    const { symbol, decimals } = await tokenMetadataFromId(wallet, chain, contract, tokenId, options);
    const flowLimitInUnits = parseTokenAmount(flowLimit, decimals);

    printInfo(`Setting flow limit for tokenId ${tokenId}`, `${flowLimit} ${symbol}`);

    const flowLimitScVal = nativeToScVal(flowLimitInUnits, { type: 'i128' });
    const operation = contract.call('set_flow_limit', hexToScVal(tokenId), flowLimitScVal);

    await broadcast(operation, wallet, chain, 'Set Flow Limit', options);
    printInfo('Successfully set flow limit', `${flowLimit} ${symbol}`);
}

async function freezeToken(wallet, _config, chain, contract, args, options) {
    const [tokenId] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
    });

    const { symbol } = await tokenMetadataFromId(wallet, chain, contract, tokenId, options);

    printInfo(`Freezing token ${symbol} (tokenId: ${tokenId})`);

    const flowLimitScVal = nativeToScVal('0', { type: 'i128' });
    const operation = contract.call('set_flow_limit', hexToScVal(tokenId), flowLimitScVal);

    await broadcast(operation, wallet, chain, 'Freeze Token', options);
    printInfo('Successfully froze token', symbol);
}

async function unfreezeToken(wallet, _config, chain, contract, args, options) {
    const [tokenId] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
    });

    const { symbol } = await tokenMetadataFromId(wallet, chain, contract, tokenId, options);

    printInfo(`Unfreezing token ${symbol} (tokenId: ${tokenId})`);

    const flowLimitScVal = nativeToScVal(null, { type: 'void' });
    const operation = contract.call('set_flow_limit', hexToScVal(tokenId), flowLimitScVal);

    await broadcast(operation, wallet, chain, 'Unfreeze Token', options);
    printInfo('Successfully unfroze token', symbol);
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

async function registerTokenMetadata(wallet, config, chain, contract, args, options) {
    const [tokenAddress] = args;
    const spender = addressToScVal(wallet.publicKey());
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;
    const destinationChain = 'axelar';

    validateParameters({
        isValidStellarAddress: { tokenAddress, gasTokenAddress },
    });

    const { gasFeeValue } = await estimateITSFee(
        chain,
        destinationChain,
        options.env,
        'TokenMetadataRegistered',
        options.gasAmount,
        config.axelar,
    );

    printInfo('Gas fee value', gasFeeValue);

    const operation = contract.call(
        'register_token_metadata',
        nativeToScVal(tokenAddress, { type: 'address' }),
        spender,
        tokenToScVal(gasTokenAddress, gasFeeValue),
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

    validateParameters({
        isValidStellarAddress: { gasTokenAddress },
        isNonEmptyString: { destinationChain, destinationTokenAddress, type },
    });
    validateChain(config.chains, destinationChain);

    const chainType = getChainConfigByAxelarId(config, destinationChain)?.chainType;
    const tokenManagerType = validateLinkType(chainType, type);

    const { gasFeeValue } = await estimateITSFee(chain, destinationChain, options.env, 'LinkToken', options.gasAmount, config.axelar);

    printInfo('Salt', salt);
    printInfo('Deployment salt (bytes32)', saltBytes32);
    printInfo('Gas fee value', gasFeeValue);

    const itsDestinationTokenAddress = encodeITSDestinationToken(config.chains, destinationChain, destinationTokenAddress);
    printInfo('Human-readable destination token address', destinationTokenAddress);

    let operatorBytes = nativeToScVal(null, { type: 'void' });
    if (options.operator) {
        printInfo('Destination Operator address', options.operator);
        operatorBytes = hexToScVal(encodeITSDestination(config.chains, destinationChain, options.operator));
    }

    const operation = contract.call(
        'link_token',
        deployer,
        hexToScVal(saltBytes32),
        nativeToScVal(destinationChain, { type: 'string' }),
        hexToScVal(itsDestinationTokenAddress),
        nativeToScVal(tokenManagerType, { type: 'u32' }),
        operatorBytes,
        tokenToScVal(gasTokenAddress, gasFeeValue),
    );

    const returnValue = await broadcast(operation, wallet, chain, 'Token Linked', options);
    printInfo('tokenId', serializeValue(returnValue.value()));
}

async function transferTokenAdmin(wallet, _config, chain, contract, args, options) {
    const [tokenId, newAdmin] = args;

    validateParameters({
        isNonEmptyString: { tokenId },
        isValidStellarAddress: { newAdmin },
    });

    const operation = contract.call('transfer_token_admin', hexToScVal(tokenId), nativeToScVal(newAdmin, { type: 'address' }));

    await broadcast(operation, wallet, chain, 'Transfer Token Admin', options);
    printInfo('New admin address', newAdmin);
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
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
            return mainProcessor(addTrustedChains, trustedChains, options);
        });

    program
        .command('remove-trusted-chains <trusted-chains...>')
        .description(`Remove trusted chains. The <trusted-chains> can be a list of chains separated by whitespaces or special tag 'all'`)
        .action((trustedChains, options) => {
            return mainProcessor(removeTrustedChains, trustedChains, options);
        });

    program
        .command('is-trusted-chain <trusted-chain>')
        .description('Check if a chain is trusted')
        .action((trustedChain, options) => {
            return mainProcessor(isTrustedChain, [trustedChain], options);
        });

    program
        .command('deploy-interchain-token <symbol> <name> <decimals> <salt> <initialSupply>')
        .description('deploy interchain token')
        .action((symbol, name, decimal, salt, initialSupply, options) => {
            return mainProcessor(deployInterchainToken, [symbol, name, decimal, salt, initialSupply], options);
        });

    program
        .command('deploy-remote-interchain-token <salt> <destinationChain>')
        .description('deploy remote interchain token')
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default('auto'))
        .action((salt, destinationChain, options) => {
            return mainProcessor(deployRemoteInterchainToken, [salt, destinationChain], options);
        });

    program
        .command('register-canonical-token <tokenAddress>')
        .description('register canonical token')
        .action((tokenAddress, options) => {
            return mainProcessor(registerCanonicalToken, [tokenAddress], options);
        });

    program
        .command('deploy-remote-canonical-token <tokenAddress> <destinationChain>')
        .description('deploy remote canonical token')
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default('auto'))
        .action((tokenAddress, destinationChain, options) => {
            return mainProcessor(deployRemoteCanonicalToken, [tokenAddress, destinationChain], options);
        });

    program
        .command('register-token-metadata <tokenAddress>')
        .description('register token metadata')
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default('auto'))
        .action((tokenAddress, options) => {
            return mainProcessor(registerTokenMetadata, [tokenAddress], options);
        });

    program
        .command('register-custom-token <salt> <tokenAddress>')
        .description('register custom token')
        .addArgument(new Argument('<type>', 'token manager type').choices(Object.keys(tokenManagerTypes)))
        .action((salt, tokenAddress, type, options) => {
            return mainProcessor(registerCustomToken, [salt, tokenAddress, type], options);
        });

    program
        .command('link-token <salt> <destinationChain> <destinationTokenAddress>')
        .description('link token')
        .addArgument(new Argument('<type>', 'token manager type').choices(Object.keys(tokenManagerTypes)))
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default('auto'))
        .addOption(new Option('--operator <operator>', 'operator for the token id on the destination chain'))
        .action((salt, destinationChain, destinationTokenAddress, type, options) => {
            return mainProcessor(linkToken, [salt, destinationChain, destinationTokenAddress, type], options);
        });

    program
        .command('interchain-transfer <tokenId> <destinationChain> <destinationAddress> <amount>')
        .description('interchain transfer (amount in token units, e.g. 100.5)')
        .addOption(new Option('--data <data>', 'data').default(''))
        .addOption(new Option('--gas-token-address <gasTokenAddress>', 'gas token address (default: XLM)'))
        .addOption(new Option('--gas-amount <gasAmount>', 'gas amount').default('auto'))
        .action((tokenId, destinationChain, destinationAddress, amount, options) => {
            return mainProcessor(interchainTransfer, [tokenId, destinationChain, destinationAddress, amount], options);
        });

    program
        .command('execute <sourceChain> <messageId> <sourceAddress> <payload>')
        .description('Execute ITS message')
        .action((sourceChain, messageId, sourceAddress, payload, options) => {
            return mainProcessor(execute, [sourceChain, messageId, sourceAddress, payload], options);
        });

    program
        .command('flow-limit <tokenId>')
        .description('Get the flow limit for a token')
        .action((tokenId, options) => {
            return mainProcessor(flowLimit, [tokenId], options);
        });

    program
        .command('set-flow-limit <tokenId> <flowLimit>')
        .description('Set the flow limit for a token (flowLimit in token units, e.g. 100.5)')
        .action((tokenId, flowLimit, options) => {
            return mainProcessor(setFlowLimit, [tokenId, flowLimit], options);
        });

    program
        .command('remove-flow-limit <tokenId>')
        .description('Remove the flow limit for a token')
        .action((tokenId, options) => {
            return mainProcessor(removeFlowLimit, [tokenId], options);
        });

    program
        .command('freeze-token <tokenId>')
        .description('Freeze a token by setting flow limit to 0')
        .action((tokenId, options) => {
            return mainProcessor(freezeToken, [tokenId], options);
        });

    program
        .command('unfreeze-token <tokenId>')
        .description('Unfreeze a token by removing the flow limit')
        .action((tokenId, options) => {
            return mainProcessor(unfreezeToken, [tokenId], options);
        });

    program
        .command('interchain-token-address <tokenId>')
        .description('Get the interchain token address with the given token id')
        .action((tokenId, options) => {
            return mainProcessor(interchainTokenAddress, [tokenId], options);
        });

    program
        .command('registered-token-address <tokenId>')
        .description('Get the registered token address for the given token id')
        .action((tokenId, options) => {
            return mainProcessor(registeredTokenAddress, [tokenId], options);
        });

    program
        .command('token-admin <tokenId>')
        .description('Get the admin address for a token with the given token id')
        .action((tokenId, options) => {
            return mainProcessor(tokenAdmin, [tokenId], options);
        });

    program
        .command('deployed-token-manager <tokenId>')
        .description('Get the deployed token manager address with the given token id')
        .action((tokenId, options) => {
            return mainProcessor(deployedTokenManager, [tokenId], options);
        });

    program
        .command('transfer-token-admin <tokenId> <newAdmin>')
        .description('Transfer admin of a token contract from token id')
        .action((tokenId, newAdmin, options) => {
            return mainProcessor(transferTokenAdmin, [tokenId, newAdmin], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parseAsync().then(() => process.exit(0));
}
module.exports = { addTrustedChains, removeTrustedChains, manageTrustedChains };
