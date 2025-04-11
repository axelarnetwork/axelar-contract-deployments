'use strict';

const { Contract, nativeToScVal, Address, Operation, rpc, authorizeInvocation, xdr } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const {
    saveConfig,
    loadConfig,
    addOptionsToCommands,
    getChainConfig,
    printInfo,
    printWarn,
    printError,
    validateParameters,
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
} = require('./utils');
const { prompt, parseTrustedChains, encodeITSDestination } = require('../common/utils');

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

    const returnValue = await broadcast(operation, wallet, chain, 'Interchain Token Deployed', options);
    printInfo('tokenId', serializeValue(returnValue.value()));
}

async function deployRemoteInterchainToken(wallet, _, chain, contract, args, options) {
    const caller = addressToScVal(wallet.publicKey());
    const [salt, destinationChain] = args;
    const saltBytes32 = saltToBytes32(salt);
    const gasTokenAddress = options.gasTokenAddress || chain.tokenAddress;
    const gasAmount = options.gasAmount;

    validateParameters({
        isValidStellarAddress: { gasTokenAddress },
        isValidNumber: { gasAmount },
    });

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

async function registerCanonicalToken(wallet, _, chain, contract, args, options) {
    const [tokenAddress] = args;

    const operation = contract.call('register_canonical_token', nativeToScVal(tokenAddress, { type: 'address' }));

    const returnValue = await broadcast(operation, wallet, chain, 'Canonical Token Registered', options);
    printInfo('tokenId', serializeValue(returnValue.value()));
}

async function deployRemoteCanonicalToken(wallet, _, chain, contract, args, options) {
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
    printInfo('Encoded ITS destination address', itsDestinationAddress);

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

async function execute(wallet, _, chain, contract, args, options) {
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

// TODO: Remove this after v1.1.1 release
async function migrateTokens(wallet, _, chain, contract, args, options) {
    let tokenIds = Array.isArray(args) ? args : [args];

    tokenIds = tokenIds.map((tokenId) => '0x'.concat(Buffer.from(tokenId, 'base64').toString('hex')));

    for (const tokenId of tokenIds) {
        printInfo('Migrating token', tokenId);
        printInfo('Upgrader address', chain.contracts.Upgrader.address);

        const tokenIdScVal = hexToScVal(tokenId);
        const upgraderAddressScVal = nativeToScVal(Address.fromString(chain.contracts.Upgrader.address), { type: 'address' });
        const newVersionScVal = nativeToScVal(options.version, { type: 'string' });

        const tokenManagerAddressOperation = contract.call('token_manager_address', tokenIdScVal);
        const tokenManagerAddressResult = await broadcast(
            tokenManagerAddressOperation,
            wallet,
            chain,
            'Retrieved TokenManager address',
            options,
        );
        const tokenManagerAddress = serializeValue(tokenManagerAddressResult.value());

        printInfo('TokenManager address', tokenManagerAddress);

        const interchainTokenAddressOperation = contract.call('interchain_token_address', tokenIdScVal);
        const interchainTokenAddressResult = await broadcast(
            interchainTokenAddressOperation,
            wallet,
            chain,
            'Retrieved InterchainToken address',
            options,
        );
        const interchainTokenAddress = serializeValue(interchainTokenAddressResult.value());

        printInfo('InterchainToken address', interchainTokenAddress);

        const auths = await createMigrateTokenAuths(
            contract,
            tokenManagerAddress,
            interchainTokenAddress,
            tokenIdScVal,
            upgraderAddressScVal,
            newVersionScVal,
            chain,
            wallet,
        );

        const operation = Operation.invokeContractFunction({
            contract: chain.contracts.InterchainTokenService.address,
            function: 'migrate_token',
            args: [tokenIdScVal, upgraderAddressScVal, newVersionScVal],
            auth: auths,
        });

        await broadcast(operation, wallet, chain, 'Migrated token', options);
    }
}

// TODO: Remove this after v1.1.1 release
async function createMigrateTokenAuths(
    contract,
    tokenManagerAddress,
    interchainTokenAddress,
    tokenIdScVal,
    upgraderAddressScVal,
    newVersionScVal,
    chain,
    wallet,
) {
    // 20 seems a reasonable number of ledgers to allow for the upgrade to take effect
    const validUntil = await new rpc.Server(chain.rpc).getLatestLedger().then((info) => info.sequence + 20);

    const walletAuth = await authorizeInvocation(
        wallet,
        validUntil,
        new xdr.SorobanAuthorizedInvocation({
            function: createAuthorizedFunc(Address.fromString(chain.contracts.InterchainTokenService.address), 'migrate_token', [
                tokenIdScVal,
                upgraderAddressScVal,
                newVersionScVal,
            ]),
            subInvocations: [],
        }),
        wallet.publicKey(),
        getNetworkPassphrase(chain.networkType),
    );

    const contractAuths = [
        createAuthorizedFunc(Address.fromString(tokenManagerAddress), 'upgrade', [
            nativeToScVal(chain.contracts.InterchainTokenService.initializeArgs.tokenManagerWasmHash),
        ]),
        createAuthorizedFunc(Address.fromString(tokenManagerAddress), 'migrate', [nativeToScVal(null)]),
        createAuthorizedFunc(Address.fromString(interchainTokenAddress), 'upgrade', [
            nativeToScVal(chain.contracts.InterchainTokenService.initializeArgs.interchainTokenWasmHash),
        ]),
        createAuthorizedFunc(Address.fromString(interchainTokenAddress), 'migrate', [nativeToScVal(null)]),
    ].map((auth, index) => {
        const sorobanAddressCredentials = new xdr.SorobanAddressCredentials({
            address: Address.fromString(chain.contracts.InterchainTokenService.address).toScAddress(),
            nonce: xdr.Int64.fromString(index.toString()),
            signatureExpirationLedger: validUntil,
            signature: xdr.ScVal.scvVec([]),
        });

        return new xdr.SorobanAuthorizationEntry({
            rootInvocation: new xdr.SorobanAuthorizedInvocation({
                function: auth,
                subInvocations: [],
            }),
            credentials: xdr.SorobanCredentials.sorobanCredentialsAddress(sorobanAddressCredentials),
        });
    });

    return [walletAuth, ...contractAuths];
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
        .command('migrate-tokens <tokenIds...>')
        .description('Migrates token TokenManagers and InterchainTokens to a new version')
        .addOption(new Option('--version <version>', 'The version to migrate to'))
        .action((tokenIds, options) => {
            mainProcessor(migrateTokens, tokenIds, options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
