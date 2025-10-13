const { Command, Option } = require('commander');
const { ITSMessageType, SUI_PACKAGE_ID, CLOCK_PACKAGE_ID, TxBuilder, copyMovePackage } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, printInfo, getChainConfig, validateParameters, encodeITSDestination } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    getUnitAmount,
    getWallet,
    findPublishedObject,
    printWalletInfo,
    getObjectIdsByObjectTypes,
    broadcastFromTxBuilder,
    moveDir,
    broadcastExecuteApprovedMessage,
    parseDiscoveryInfo,
    parseGatewayInfo,
    checkTrustedAddresses,
} = require('./utils');
const { ethers } = require('hardhat');
const {
    utils: { defaultAbiCoder, keccak256, toUtf8Bytes, hexlify, randomBytes },
} = ethers;

async function sendToken(keypair, client, contracts, args, options) {
    const config = loadConfig(options.env);
    const { Example, GasService, AxelarGateway, InterchainTokenService } = contracts;
    const [symbol, destinationChain, destinationAddress, feeAmount, amount] = args;

    validateParameters({
        isValidNumber: { amount, feeAmount },
    });

    const destinationAddressEnc = encodeITSDestination(config.chains, destinationChain, destinationAddress);

    const ItsToken = contracts[symbol.toUpperCase()];

    if (!ItsToken) {
        throw new Error(`Token ${symbol} not found. Deploy it first with 'ts-node sui/its-example.js deploy-token' command`);
    }

    await checkTrustedAddresses(destinationChain);

    const decimals = ItsToken.decimals;

    const unitAmount = getUnitAmount(amount, decimals);
    const unitFeeAmount = getUnitAmount(feeAmount);
    const walletAddress = keypair.toSuiAddress();

    const objectIds = {
        singleton: Example.objects.ItsSingleton,
        its: InterchainTokenService.objects.InterchainTokenService,
        gateway: AxelarGateway.objects.Gateway,
        gasService: GasService.objects.GasService,
    };

    const txBuilder = new TxBuilder(client);

    const tx = txBuilder.tx;
    const gas = tx.splitCoins(tx.gas, [unitFeeAmount]);

    const TokenId = await txBuilder.moveCall({
        target: `${InterchainTokenService.address}::token_id::from_u256`,
        arguments: [ItsToken.objects.TokenId],
    });

    let Coin;
    const tokenManagerMode = ItsToken.tokenManagerMode || 'lock_unlock';
    const isOrigin = ItsToken.objects.origin;

    if (isOrigin && tokenManagerMode === 'mint_burn') {
        // For origin tokens with mint_burn mode, TreasuryCap is transferred to InterchainTokenService
        // Use existing coins from wallet instead of minting
        const coins = await client.getCoins({
            owner: walletAddress,
            coinType: ItsToken.typeArgument,
        });

        if (!coins.data.length) {
            throw new Error(
                `No ${symbol} tokens found in wallet. Make sure tokens were minted during deployment or use mint-token command.`,
            );
        }

        // Find a coin with sufficient balance or merge coins
        let selectedCoin = null;
        let totalBalance = 0n;

        for (const coin of coins.data) {
            totalBalance += BigInt(coin.balance);
            if (BigInt(coin.balance) >= BigInt(unitAmount)) {
                selectedCoin = coin;
                break;
            }
        }

        if (totalBalance < BigInt(unitAmount)) {
            throw new Error(`Insufficient ${symbol} balance. Required: ${unitAmount}, Available: ${totalBalance}`);
        }

        if (selectedCoin && BigInt(selectedCoin.balance) >= BigInt(unitAmount)) {
            // Use existing coin directly if it has enough balance
            if (BigInt(selectedCoin.balance) > BigInt(unitAmount)) {
                // Split the coin to get exact amount
                const coinObject = tx.object(selectedCoin.coinObjectId);
                Coin = tx.splitCoins(coinObject, [unitAmount]);
            } else {
                // Use the whole coin
                Coin = tx.object(selectedCoin.coinObjectId);
            }
        } else {
            // Merge multiple coins to get the required amount
            const coinObjects = coins.data.map((coin) => tx.object(coin.coinObjectId));
            const primaryCoin = coinObjects[0];

            if (coinObjects.length > 1) {
                tx.mergeCoins(primaryCoin, coinObjects.slice(1));
            }

            Coin = tx.splitCoins(primaryCoin, [unitAmount]);
        }
    } else {
        // For non-origin tokens or lock_unlock mode, mint new coins using TreasuryCap
        Coin = await txBuilder.moveCall({
            target: `${SUI_PACKAGE_ID}::coin::mint`,
            arguments: [ItsToken.objects.TreasuryCap, unitAmount],
            typeArguments: [ItsToken.typeArgument],
        });
    }

    await txBuilder.moveCall({
        target: `${Example.address}::its::send_interchain_transfer_call`,
        arguments: [
            objectIds.singleton,
            objectIds.its,
            objectIds.gateway,
            objectIds.gasService,
            TokenId,
            Coin,
            destinationChain,
            destinationAddressEnc,
            '0x', // its token metadata
            walletAddress,
            gas,
            '0x', // gas params
            CLOCK_PACKAGE_ID,
        ],
        typeArguments: [ItsToken.typeArgument],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, `${amount} ${symbol} Token Sent`, options);
}

async function sendDeployment(keypair, client, contracts, args, options) {
    const { AxelarGateway, GasService, InterchainTokenService, Example } = contracts;
    const [symbol, destinationChain, feeAmount] = args;
    const Token = contracts[symbol.toUpperCase()];
    const feeUnitAmount = getUnitAmount(feeAmount);

    await checkTrustedAddresses(destinationChain);

    const txBuilder = new TxBuilder(client);

    const tx = txBuilder.tx;
    const gas = tx.splitCoins(tx.gas, [feeUnitAmount]);

    const TokenId = await txBuilder.moveCall({
        target: `${InterchainTokenService.address}::token_id::from_u256`,
        arguments: [Token.objects.TokenId],
    });

    await txBuilder.moveCall({
        target: `${Example.address}::its::deploy_remote_interchain_token`,
        arguments: [
            InterchainTokenService.objects.InterchainTokenService,
            AxelarGateway.objects.Gateway,
            GasService.objects.GasService,
            destinationChain,
            TokenId,
            gas,
            '0x',
            keypair.toSuiAddress(),
        ],
        typeArguments: [Token.typeArgument],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, `Sent ${symbol} Deployment on ${destinationChain}`, options);
}

async function handleReceivedMessage(keypair, client, contracts, args, options, actionName) {
    const { InterchainTokenService } = contracts;
    const [sourceChain, messageId, sourceAddress, tokenSymbol, payload] = args;

    // Prepare Object Ids
    const symbol = tokenSymbol.toUpperCase();

    if (!contracts[symbol]) {
        throw new Error(`Token ${symbol} not found. Deploy it first with 'ts-node sui/its-example.js deploy-token' command`);
    }

    const discoveryInfo = parseDiscoveryInfo(contracts);
    const gatewayInfo = parseGatewayInfo(contracts);
    const messageInfo = {
        source_chain: sourceChain,
        message_id: messageId,
        source_address: sourceAddress,
        destination_id: InterchainTokenService.objects.ChannelId,
        payload,
    };

    await broadcastExecuteApprovedMessage(client, keypair, discoveryInfo, gatewayInfo, messageInfo, actionName, options);
}

async function receiveToken(keypair, client, contracts, args, options) {
    const symbol = args[3];
    await handleReceivedMessage(keypair, client, contracts, args, options, `${symbol} Token Received`);
}

async function receiveDeployment(keypair, client, contracts, args, options) {
    const symbol = args[3];
    await handleReceivedMessage(keypair, client, contracts, args, options, `Received ${symbol} Token Deployment`);
}

async function deployToken(keypair, client, contracts, args, options) {
    const [symbol, name, decimals] = args;

    const walletAddress = keypair.toSuiAddress();
    copyMovePackage('interchain_token', null, moveDir);
    // Define the interchain token options
    const interchainTokenOptions = {
        symbol,
        name,
        decimals,
    };

    // Publish the interchain token
    const txBuilder = new TxBuilder(client);

    const cap = await txBuilder.publishInterchainToken(moveDir, interchainTokenOptions);

    txBuilder.tx.transferObjects([cap], walletAddress);

    const publishTxn = await broadcastFromTxBuilder(txBuilder, keypair, `Published ${symbol}`, options);

    const publishObject = findPublishedObject(publishTxn);

    const packageId = publishObject.packageId;
    const tokenType = `${packageId}::${symbol.toLowerCase()}::${symbol.toUpperCase()}`;

    const [TreasuryCap, Metadata] = getObjectIdsByObjectTypes(publishTxn, [`TreasuryCap<${tokenType}>`, `Metadata<${tokenType}>`]);

    // Register Token in InterchainTokenService
    const { Example, InterchainTokenService } = contracts;
    let tokenId;

    const postDeployTxBuilder = new TxBuilder(client);

    if (options.origin) {
        // Mint tokens before registration (while user still holds the TreasuryCap)
        const amount = !isNaN(options.mintAmount) ? parseInt(options.mintAmount) : 0;
        if (amount) {
            const unitAmount = getUnitAmount(options.mintAmount, decimals);

            const mintTxBuilder = new TxBuilder(client);

            const coin = await mintTxBuilder.moveCall({
                target: `${SUI_PACKAGE_ID}::coin::mint`,
                arguments: [TreasuryCap, unitAmount],
                typeArguments: [tokenType],
            });

            mintTxBuilder.tx.transferObjects([coin], walletAddress);

            await broadcastFromTxBuilder(mintTxBuilder, keypair, `Minted ${amount} ${symbol}`, options);
        }

        if (options.tokenManagerMode === 'lock_unlock') {
            await postDeployTxBuilder.moveCall({
                target: `${Example.address}::its::register_coin`,
                arguments: [InterchainTokenService.objects.InterchainTokenService, Metadata],
                typeArguments: [tokenType],
            });
        } else {
            await postDeployTxBuilder.moveCall({
                target: `${Example.address}::its::register_coin_with_cap`,
                arguments: [InterchainTokenService.objects.InterchainTokenService, Metadata, TreasuryCap],
                typeArguments: [tokenType],
            });
        }

        const result = await broadcastFromTxBuilder(
            postDeployTxBuilder,
            keypair,
            `Setup ${symbol} as an origin in InterchainTokenService successfully`,
            options,
            {
                showEvents: true,
            },
        );
        tokenId = result.events[0].parsedJson.token_id.id;
    } else {
        await postDeployTxBuilder.moveCall({
            target: `${InterchainTokenService.address}::interchain_token_service::give_unregistered_coin`,
            arguments: [InterchainTokenService.objects.InterchainTokenService, TreasuryCap, Metadata],
            typeArguments: [tokenType],
        });
        await broadcastFromTxBuilder(
            postDeployTxBuilder,
            keypair,
            `Setup ${symbol} as a non-origin in InterchainTokenService successfully`,
            options,
            {
                showEvents: true,
            },
        );
    }

    // Save the deployed token info in the contracts object.
    contracts[symbol.toUpperCase()] = {
        address: packageId,
        typeArgument: tokenType,
        decimals,
        objects: {
            TreasuryCap,
            Metadata,
            TokenId: tokenId,
            origin: options.origin,
        },
        tokenManagerMode: options.tokenManagerMode || 'lock_unlock', // default to lock_unlock
    };
}

async function printReceiveDeploymentInfo(contracts, args, options) {
    const [sourceChain, name, symbol, decimals] = args;

    const messageType = ITSMessageType.InterchainTokenDeployment;
    const tokenId = options.tokenId;
    const byteName = toUtf8Bytes(name);
    const byteSymbol = toUtf8Bytes(symbol);
    const tokenDecimals = parseInt(decimals);
    const tokenDistributor = options.distributor;

    // InterchainTokenService transfer payload from Ethereum to Sui
    const itsMessage = defaultAbiCoder.encode(
        ['uint256', 'uint256', 'bytes', 'bytes', 'uint256', 'bytes'],
        [messageType, tokenId, byteName, byteSymbol, tokenDecimals, tokenDistributor],
    );
    const hubMessage = defaultAbiCoder.encode(['uint256', 'string', 'bytes'], [ITSMessageType.ReceiveFromItsHub, sourceChain, itsMessage]);

    printInfo(
        JSON.stringify(
            {
                payload: hubMessage,
                tokenId,
                payloadHash: keccak256(hubMessage),
            },
            null,
            2,
        ),
    );
}

async function printReceiveTransferInfo(contracts, args, options) {
    const { Example } = contracts;
    const [sourceChain, symbol, sourceAddress, amount] = args;

    const Token = contracts[symbol];
    const unitAmount = getUnitAmount(amount, Token.decimals);
    const tokenId = Token.objects.TokenId;
    const itsBytes = options.itsBytes;
    const channelId = options.channelId || Example.objects.ItsChannelId;

    const itsMessage = defaultAbiCoder.encode(
        ['uint256', 'uint256', 'bytes', 'bytes', 'uint256', 'bytes'],
        [ITSMessageType.InterchainTokenTransfer, tokenId, sourceAddress, channelId, unitAmount, itsBytes],
    );
    const hubMessage = defaultAbiCoder.encode(['uint256', 'string', 'bytes'], [ITSMessageType.ReceiveFromItsHub, sourceChain, itsMessage]);

    printInfo(
        JSON.stringify(
            {
                payload: hubMessage,
                tokenId,
                payloadHash: keccak256(hubMessage),
            },
            null,
            2,
        ),
    );
}

async function mintToken(keypair, client, contracts, args, options) {
    const [symbol] = args;
    const amount = options.amount;
    const recipient = options.recipient || keypair.toSuiAddress();
    const Token = contracts[symbol.toUpperCase()];
    const unitAmount = getUnitAmount(amount, Token.decimals);

    const mintTxBuilder = new TxBuilder(client);

    const coin = await mintTxBuilder.moveCall({
        target: `${SUI_PACKAGE_ID}::coin::mint`,
        arguments: [Token.objects.TreasuryCap, unitAmount],
        typeArguments: [Token.typeArgument],
    });

    mintTxBuilder.tx.transferObjects([coin], recipient);

    await broadcastFromTxBuilder(mintTxBuilder, keypair, `Minted ${amount} ${symbol}`, options);
}

async function processCommand(command, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    await command(keypair, client, chain.contracts, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(command, chain, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('InterchainTokenService Example').description('SUI InterchainTokenService Example scripts');

    const sendTokenTransferProgram = new Command()
        .name('send-token')
        .description('Send token from Sui to other chain.')
        .command('send-token <symbol> <dest-chain> <dest-contract-address> <fee> <amount>')
        .action((symbol, destChain, destContractAddress, feeAmount, amount, options) => {
            mainProcessor(sendToken, options, [symbol, destChain, destContractAddress, feeAmount, amount], processCommand);
        });

    const receiveTokenTransferProgram = new Command()
        .name('receive-token')
        .description('Receive token from other chain to Sui.')
        .command('receive-token <source-chain> <message-id> <source-address> <token-symbol> <payload>')
        .addOption(new Option('--data <data>', 'Data').default(ethers.constants.HashZero))
        .action((sourceChain, messageId, sourceAddress, tokenSymbol, payload, options) => {
            mainProcessor(receiveToken, options, [sourceChain, messageId, sourceAddress, tokenSymbol, payload], processCommand);
        });

    const deployTokenProgram = new Command()
        .name('deploy-token')
        .description('Deploy token on Sui. The supported token manager modes are lock_unlock (default) and mint_burn.')
        .command('deploy-token <symbol> <name> <decimals>')
        .addOption(
            new Option('--tokenManagerMode <tokenManagerMode>', 'Token Manager Mode')
                .default('lock_unlock')
                .choices(['lock_unlock', 'mint_burn']),
        )
        .addOption(new Option('--mintAmount <amount>', 'Amount of tokens to mint to the deployer (must be origin).').default('1000'))
        .addOption(new Option('--origin', 'Deploy as a origin token or receive deployment from another chain', false))
        .action((symbol, name, decimals, options) => {
            mainProcessor(deployToken, options, [symbol, name, decimals], processCommand);
        });

    const sendTokenDeploymentProgram = new Command()
        .name('send-deployment')
        .description('Send token deployment from Sui to other chain.')
        .command('send-deployment <symbol> <destination-chain> <fee>')
        .action((symbol, destinationChain, fee, options) => {
            mainProcessor(sendDeployment, options, [symbol, destinationChain, fee], processCommand);
        });

    // The token must be deployed on sui first before executing receive deployment command
    // and the token must have zero supply, otherwise the command will fail.
    // To deploy the token, use the command `ts-node sui/its-example.js deploy-token <symbol> <name> <decimals>`
    const receiveTokenDeploymentProgram = new Command()
        .name('receive-deployment')
        .description('Receive token deployment from other chain to Sui.')
        .command('receive-deployment <source-chain> <message-id> <source-address> <token-symbol> <payload>')
        .action((symbol, sourceChain, messageId, sourceAddress, payload, options) => {
            mainProcessor(receiveDeployment, options, [symbol, sourceChain, messageId, sourceAddress, payload], processCommand);
        });

    const mintTokenProgram = new Command()
        .name('mint-token')
        .description('Mint token for the given symbol on Sui. The token must be deployed on Sui first.')
        .command('mint-token <symbol>')
        .addOption(new Option('--recipient <recipient>', 'Recipient address'))
        .addOption(new Option('--amount <amount>', 'Amount to mint').default('1000'))
        .action((symbol, options) => {
            mainProcessor(mintToken, options, [symbol], processCommand);
        });

    const printDeploymentInfoProgram = new Command()
        .name('print-deployment-info')
        .description('Print deployment info. This script will be useful for testing receive deployment flow.')
        .command('print-receive-deployment <sourceChain> <name> <symbol> <decimals>')
        .addOption(new Option('--distributor <distributor>', 'Distributor address').default(ethers.constants.HashZero))
        .addOption(new Option('--tokenId <tokenId>', 'Token ID').default(hexlify(randomBytes(32))))
        .action((sourceChain, name, symbol, decimals, options) => {
            const config = loadConfig(options.env);
            const chain = getChainConfig(config.chains, options.chainName);
            printReceiveDeploymentInfo(chain.contracts, [sourceChain, name, symbol, decimals], options);
        });

    const printReceiveTransferInfoProgram = new Command()
        .name('print-transfer-info')
        .description('Print receive token info. This script will be useful for testing receive token flow.')
        .command('print-receive-transfer <sourceChain> <symbol> <source-address> <amount>')
        .addOption(new Option('--itsBytes <itsBytes>', 'InterchainTokenService Bytes').default(ethers.constants.HashZero))
        .action((sourceChain, symbol, sourceAddress, amount, options) => {
            const config = loadConfig(options.env);
            const chain = getChainConfig(config.chains, options.chainName);
            printReceiveTransferInfo(chain.contracts, [sourceChain, symbol, sourceAddress, amount], options);
        });

    program.addCommand(sendTokenTransferProgram);
    program.addCommand(receiveTokenTransferProgram);
    program.addCommand(deployTokenProgram);
    program.addCommand(sendTokenDeploymentProgram);
    program.addCommand(receiveTokenDeploymentProgram);
    program.addCommand(mintTokenProgram);
    program.addCommand(printDeploymentInfoProgram);
    program.addCommand(printReceiveTransferInfoProgram);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
