'use strict';

const { Contract, nativeToScVal, Address } = require('@stellar/stellar-sdk');
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

async function migrateToken(wallet, _, chain, contract, args, options) {
    const [tokenId, version] = args;

    const tokenIdScVal = nativeToScVal(Buffer.from(tokenId, 'hex'));
    const upgraderAddressScVal = nativeToScVal(Address.fromString(chain.contracts.Upgrader.address), { type: 'address' });
    const newVersionScVal = nativeToScVal(version, { type: 'string' });

    const operation = contract.call('migrate_token', tokenIdScVal, upgraderAddressScVal, newVersionScVal);
    await broadcast(operation, wallet, chain, 'Migrated token', options);
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
        .command('migrate-token <tokenId> <version>')
        .description("Migrates a token's TokenManager and InterchainToken to a new version")
        .addOption(
            new Option(
                '--tokenId <tokenId>',
                'The tokenId to migrate in base64 format (example: "Ti+Y+1GPlMl6ZvSfJSq1lTJna8pWcboxzVkujlT0/F0="',
            ),
        )
        .addOption(new Option('--version <version>', 'The version to migrate to'))
        .action((tokenId, version, options) => {
            mainProcessor(migrateToken, [tokenId, version], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
