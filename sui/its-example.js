const { Command, Option } = require('commander');
const { ITSMessageType, SUI_PACKAGE_ID, CLOCK_PACKAGE_ID, TxBuilder, copyMovePackage } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, printInfo, getChainConfig } = require('../common/utils');
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
    checkTrustedAddresses,
    parseDiscoveryInfo,
    parseGatewayInfo,
} = require('./utils');
const { ethers } = require('hardhat');
const {
    utils: { defaultAbiCoder, keccak256, toUtf8Bytes, hexlify, randomBytes },
} = ethers;

async function sendToken(keypair, client, contracts, args, options) {
    const [symbol, destinationChain, destinationAddress, feeAmount, amount] = args;

    const { Example, GasService, AxelarGateway, ITS } = contracts;
    const ItsToken = contracts[symbol.toUpperCase()];

    if (!ItsToken) {
        throw new Error(`Token ${symbol} not found. Deploy it first with 'node sui/its-example.js deploy-token' command`);
    }

    checkTrustedAddresses(ITS.trustedAddresses, destinationChain);

    const decimals = ItsToken.decimals;

    const unitAmount = getUnitAmount(amount, decimals);
    const unitFeeAmount = getUnitAmount(feeAmount);
    const walletAddress = keypair.toSuiAddress();

    const objectIds = {
        singleton: Example.objects.ItsSingleton,
        its: ITS.objects.ITS,
        gateway: AxelarGateway.objects.Gateway,
        gasService: GasService.objects.GasService,
    };

    const txBuilder = new TxBuilder(client);

    const tx = txBuilder.tx;

    const gas = tx.splitCoins(tx.gas, [unitFeeAmount]);

    const TokenId = await txBuilder.moveCall({
        target: `${ITS.address}::token_id::from_u256`,
        arguments: [ItsToken.objects.TokenId],
    });

    const Coin = await txBuilder.moveCall({
        target: `${SUI_PACKAGE_ID}::coin::mint`,
        arguments: [ItsToken.objects.TreasuryCap, unitAmount],
        typeArguments: [ItsToken.typeArgument],
    });

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
            destinationAddress,
            '0x', // its token metadata
            walletAddress,
            gas,
            '0x', // gas params
            CLOCK_PACKAGE_ID,
        ],
        typeArguments: [ItsToken.typeArgument],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, `${amount} ${symbol} Token Sent`);
}

async function sendDeployment(keypair, client, contracts, args, options) {
    const { AxelarGateway, GasService, ITS, Example } = contracts;
    const [symbol, destinationChain, feeAmount] = args;
    const Token = contracts[symbol.toUpperCase()];
    const feeUnitAmount = getUnitAmount(feeAmount);

    checkTrustedAddresses(ITS.trustedAddresses, destinationChain);

    const txBuilder = new TxBuilder(client);

    const tx = txBuilder.tx;
    const gas = tx.splitCoins(tx.gas, [feeUnitAmount]);

    const TokenId = await txBuilder.moveCall({
        target: `${ITS.address}::token_id::from_u256`,
        arguments: [Token.objects.TokenId],
    });

    await txBuilder.moveCall({
        target: `${Example.address}::its::deploy_remote_interchain_token`,
        arguments: [
            ITS.objects.ITS,
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

    await broadcastFromTxBuilder(txBuilder, keypair, `Sent ${symbol} Deployment on ${destinationChain}`);
}

async function handleReceivedMessage(keypair, client, contracts, args, options, actionName) {
    const { ITS } = contracts;
    const [sourceChain, messageId, sourceAddress, tokenSymbol, payload] = args;

    checkTrustedAddresses(ITS.trustedAddresses, sourceChain);

    // Prepare Object Ids
    const symbol = tokenSymbol.toUpperCase();

    if (!contracts[symbol]) {
        throw new Error(`Token ${symbol} not found. Deploy it first with 'node sui/its-example.js deploy-token' command`);
    }

    const discoveryInfo = parseDiscoveryInfo(contracts);
    const gatewayInfo = parseGatewayInfo(contracts);
    const messageInfo = {
        source_chain: sourceChain,
        message_id: messageId,
        source_address: sourceAddress,
        destination_id: ITS.objects.ChannelId,
        payload,
    };

    await broadcastExecuteApprovedMessage(client, keypair, discoveryInfo, gatewayInfo, messageInfo, actionName);
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

    const publishTxn = await broadcastFromTxBuilder(txBuilder, keypair, `Published ${symbol}`);

    const publishObject = findPublishedObject(publishTxn);

    const packageId = publishObject.packageId;
    const tokenType = `${packageId}::${symbol.toLowerCase()}::${symbol.toUpperCase()}`;

    const [TreasuryCap, Metadata] = getObjectIdsByObjectTypes(publishTxn, [`TreasuryCap<${tokenType}>`, `Metadata<${tokenType}>`]);

    // Register Token in ITS
    const { Example, ITS } = contracts;
    let tokenId;

    const postDeployTxBuilder = new TxBuilder(client);

    if (options.origin) {
        await postDeployTxBuilder.moveCall({
            target: `${Example.address}::its::register_coin`,
            arguments: [ITS.objects.ITS, Metadata],
            typeArguments: [tokenType],
        });
        const result = await broadcastFromTxBuilder(postDeployTxBuilder, keypair, `Setup ${symbol} as an origin in ITS successfully`, {
            showEvents: true,
        });
        tokenId = result.events[0].parsedJson.token_id.id;
    } else {
        await postDeployTxBuilder.moveCall({
            target: `${ITS.address}::its::give_unregistered_coin`,
            arguments: [ITS.objects.ITS, TreasuryCap, Metadata],
            typeArguments: [tokenType],
        });
        await broadcastFromTxBuilder(postDeployTxBuilder, keypair, `Setup ${symbol} as a non-origin in ITS successfully`, {
            showEvents: true,
        });
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
    };
}

async function printReceiveDeploymentInfo(contracts, args, options) {
    const [name, symbol, decimals] = args;

    const messageType = ITSMessageType.InterchainTokenDeployment;
    const tokenId = options.tokenId;
    const byteName = toUtf8Bytes(name);
    const byteSymbol = toUtf8Bytes(symbol);
    const tokenDecimals = parseInt(decimals);
    const tokenDistributor = options.distributor;

    // ITS transfer payload from Ethereum to Sui
    const payload = defaultAbiCoder.encode(
        ['uint256', 'uint256', 'bytes', 'bytes', 'uint256', 'bytes'],
        [messageType, tokenId, byteName, byteSymbol, tokenDecimals, tokenDistributor],
    );

    printInfo(
        JSON.stringify(
            {
                payload,
                tokenId,
                payloadHash: keccak256(payload),
            },
            null,
            2,
        ),
    );
}

async function printReceiveTransferInfo(contracts, args, options) {
    const { Example } = contracts;
    const [symbol, sourceAddress, amount] = args;

    const Token = contracts[symbol];
    const unitAmount = getUnitAmount(amount, Token.decimals);
    const tokenId = Token.objects.TokenId;
    const itsBytes = options.itsBytes;
    const channelId = options.channelId || Example.objects.ItsChannelId;

    const payload = defaultAbiCoder.encode(
        ['uint256', 'uint256', 'bytes', 'bytes', 'uint256', 'bytes'],
        [ITSMessageType.InterchainTokenTransfer, tokenId, sourceAddress, channelId, unitAmount, itsBytes],
    );

    printInfo(
        JSON.stringify(
            {
                payload,
                tokenId,
                payloadHash: keccak256(payload),
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

    await broadcastFromTxBuilder(mintTxBuilder, keypair, `Minted ${amount} ${symbol}`);
}

async function processCommand(command, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    await command(keypair, client, chain.contracts, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(command, chain, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('ITS Example').description('SUI ITS Example scripts');

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
        .description('Deploy token on Sui.')
        .command('deploy-token <symbol> <name> <decimals>')
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
    // To deploy the token, use the command `node sui/its-example.js deploy-token <symbol> <name> <decimals>`
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
        .command('print-receive-deployment <name> <symbol> <decimals>')
        .addOption(new Option('--distributor <distributor>', 'Distributor address').default(ethers.constants.HashZero))
        .addOption(new Option('--tokenId <tokenId>', 'Token ID').default(hexlify(randomBytes(32))))
        .action((name, symbol, decimals, options) => {
            const config = loadConfig(options.env);
            const chain = getChainConfig(config, options.chainName);
            printReceiveDeploymentInfo(chain.contracts, [name, symbol, decimals], options);
        });

    const printReceiveTransferInfoProgram = new Command()
        .name('print-transfer-info')
        .description('Print receive token info. This script will be useful for testing receive token flow.')
        .command('print-receive-transfer <symbol> <source-address> <amount>')
        .addOption(new Option('--itsBytes <itsBytes>', 'ITS Bytes').default(ethers.constants.HashZero))
        .action((symbol, sourceAddress, amount, options) => {
            const config = loadConfig(options.env);
            const chain = getChainConfig(config, options.chainName);
            printReceiveTransferInfo(chain.contracts, [symbol, sourceAddress, amount], options);
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
