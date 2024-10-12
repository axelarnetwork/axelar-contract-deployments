const { Command, Option } = require('commander');
const { ITSMessageType, SUI_PACKAGE_ID, CLOCK_PACKAGE_ID, TxBuilder, copyMovePackage } = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, printInfo } = require('../common/utils');
const {
    parseExecuteDataFromTransaction,
    addBaseOptions,
    addOptionsToCommands,
    getUnitAmount,
    getWallet,
    findPublishedObject,
    printWalletInfo,
    getObjectIdsByObjectTypes,
    broadcastFromTxBuilder,
    moveDir,
    getTransactionList,
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

async function receiveToken(keypair, client, contracts, args, options) {
    const itsData = options.data || ethers.constants.HashZero;
    const { Example, RelayerDiscovery, AxelarGateway, ITS } = contracts;
    const [sourceChain, messageId, sourceAddress, tokenSymbol, amount] = args;

    // Prepare Object Ids
    const symbol = tokenSymbol.toUpperCase();
    const ids = {
        discovery: RelayerDiscovery.objects.RelayerDiscoveryv0,
        gateway: AxelarGateway.objects.Gateway,
        itsChannel: ITS.objects.ChannelId,
        exampleChannel: options.channelId || Example.objects.ItsChannelId,
    };

    if (!ids.exampleChannel) {
        throw new Error('Please provide either a channel id (--channelId) or deploy the Example contract');
    }

    if (!contracts[symbol]) {
        throw new Error(`Token ${symbol} not found. Deploy it first with 'node sui/its-example.js deploy-token' command`);
    }

    const unitAmount = getUnitAmount(amount, contracts[symbol].decimals);
    const Token = contracts[symbol];
    const tokenId = Token.objects.TokenId;

    const payload = defaultAbiCoder.encode(
        ['uint256', 'uint256', 'bytes', 'bytes', 'uint256', 'bytes'],
        [ITSMessageType.InterchainTokenTransfer, tokenId, sourceAddress, ids.exampleChannel, unitAmount, itsData],
    );

    // To check with the payload hash from the approve command for debugging
    printInfo('Payload Hash', keccak256(payload));

    // Get Discovery table id from discovery object
    const transactionList = await getTransactionList(client, ids.discovery);

    // Find the transaction with associated channel id
    const transaction = transactionList.find((row) => row.name.value === ids.exampleChannel);

    const receiveTxBuilder = new TxBuilder(client);

    // Take the approved message from the gateway contract.
    const approvedMessage = await receiveTxBuilder.moveCall({
        target: `${AxelarGateway.address}::gateway::take_approved_message`,
        arguments: [ids.gateway, sourceChain, messageId, sourceAddress, ids.itsChannel, payload],
    });

    const { moduleName, name, packageId, txArgs } = await parseExecuteDataFromTransaction(client, transaction, approvedMessage);

    // Execute the move call dynamically based on the transaction object
    await receiveTxBuilder.moveCall({
        target: `${packageId}::${moduleName}::${name}`,
        arguments: txArgs,
        typeArguments: [Token.typeArgument],
    });

    await broadcastFromTxBuilder(receiveTxBuilder, keypair, `${symbol} Token Received`);
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

    if (!options.skipRegister) {
        const registerTxBuilder = new TxBuilder(client);

        await registerTxBuilder.moveCall({
            target: `${Example.address}::its::register_coin`,
            arguments: [ITS.objects.ITS, Metadata],
            typeArguments: [tokenType],
        });

        const result = await broadcastFromTxBuilder(registerTxBuilder, keypair, `Registered ${symbol} in ITS`, { showEvents: true });
        tokenId = result.events[0].parsedJson.token_id.id;
    } else {
        printInfo(`Skipped registering ${symbol} in ITS`);
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
        },
    };

    // Mint Token
    if (!options.skipMint) {
        const mintTxBuilder = new TxBuilder(client);

        const coin = await mintTxBuilder.moveCall({
            target: `${SUI_PACKAGE_ID}::coin::mint`,
            arguments: [TreasuryCap, getUnitAmount('1000', decimals)],
            typeArguments: [tokenType],
        });

        mintTxBuilder.tx.transferObjects([coin], walletAddress);

        await broadcastFromTxBuilder(mintTxBuilder, keypair, `Minted 1,000 ${symbol}`);
    }
}

async function sendDeployment(keypair, client, contracts, args, options) {
    const { AxelarGateway, GasService, ITS, Example } = contracts;
    const [symbol, destinationChain, destinationITSAddress, feeAmount] = args;
    const Token = contracts[symbol.toUpperCase()];
    const feeUnitAmount = getUnitAmount(feeAmount);

    const txBuilder = new TxBuilder(client);

    const tx = txBuilder.tx;
    const gas = tx.splitCoins(tx.gas, [feeUnitAmount]);

    if (!ITS.trustedAddresses[destinationChain] || !ITS.trustedAddresses[destinationChain].includes(destinationITSAddress)) {
        throw new Error(
            `Destination address ${destinationITSAddress} is not trusted on ${destinationChain}. Check if the given adress is trusted on ${destinationChain} or set trusted address with 'node sui/its-example.js setup-trusted-address <destination-chain> <destination-address>'`,
        );
    }

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

async function printDeploymentInfo(contracts, args, options) {
    const [name, symbol, decimals] = args;

    const byteName = toUtf8Bytes(name);
    const byteSymbol = toUtf8Bytes(symbol);
    const tokenDecimals = parseInt(decimals);
    const tokenId = options.tokenId;
    const tokenDistributor = options.distributor;

    // ITS transfer payload from Ethereum to Sui
    const payload = defaultAbiCoder.encode(
        ['uint256', 'uint256', 'bytes', 'bytes', 'uint256', 'bytes'],
        [ITSMessageType.InterchainTokenDeployment, tokenId, byteName, byteSymbol, tokenDecimals, tokenDistributor],
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

async function receiveDeployment(keypair, client, contracts, args, options) {
    const [symbol, sourceChain, messageId, sourceAddress, destinationContractAddress, payload] = args;

    const { AxelarGateway, ITS } = contracts;
    const Token = contracts[symbol.toUpperCase()];

    const txBuilder = new TxBuilder(client);

    const approvedMessage = await txBuilder.moveCall({
        target: `${AxelarGateway.address}::gateway::take_approved_message`,
        arguments: [AxelarGateway.objects.Gateway, sourceChain, messageId, sourceAddress, destinationContractAddress, payload],
    });

    await txBuilder.moveCall({
        target: `${ITS.address}::its::give_unregistered_coin`,
        arguments: [ITS.objects.ITS, Token.objects.TreasuryCap, Token.objects.Metadata],
        typeArguments: [Token.typeArgument],
    });

    await txBuilder.moveCall({
        target: `${ITS.address}::its::receive_deploy_interchain_token`,
        arguments: [ITS.objects.ITS, approvedMessage],
        typeArguments: [Token.typeArgument],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, `Received ${symbol} Token Deployment`);
}

async function setupTrustedAddress(keypair, client, contracts, args, options) {
    const [trustedChain, trustedAddress] = args;

    const { ITS: itsConfig } = contracts;

    const { OwnerCap, ITS } = itsConfig.objects;

    const txBuilder = new TxBuilder(client);

    const trustedAddressesObject = await txBuilder.moveCall({
        target: `${itsConfig.address}::trusted_addresses::new`,
        arguments: [[trustedChain], [trustedAddress]],
    });

    await txBuilder.moveCall({
        target: `${itsConfig.address}::its::set_trusted_addresses`,
        arguments: [ITS, OwnerCap, trustedAddressesObject],
    });

    await broadcastFromTxBuilder(txBuilder, keypair, 'Setup Trusted Address');

    // Add trusted address to ITS config
    if (!contracts.ITS.trustedAddresses) contracts.ITS.trustedAddresses = {};
    if (!contracts.ITS.trustedAddresses[trustedChain]) contracts.ITS.trustedAddresses[trustedChain] = [];

    contracts.ITS.trustedAddresses[trustedChain].push(trustedAddress);
}

async function mintToken(keypair, client, contracts, args, options) {}

async function processCommand(command, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    //const contracts = [chain.contracts.Example, chain.contracts.GasService, chain.contracts.AxelarGateway, chain.contracts.ITS];

    await command(keypair, client, chain.contracts, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    await processor(command, config.sui, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('ITS').description('SUI ITS scripts');

    const sendTokenTransferProgram = new Command()
        .name('send-token')
        .description('Send token')
        .command('send-token <symbol> <dest-chain> <dest-contract-address> <fee> <amount>')
        .action((symbol, destChain, destContractAddress, feeAmount, amount, options) => {
            mainProcessor(sendToken, options, [symbol, destChain, destContractAddress, feeAmount, amount], processCommand);
        });

    const receiveTokenTransferProgram = new Command()
        .name('receive-token')
        .description('Receive token')
        .command('receive-token <source-chain> <message-id> <source-address> <token-symbol> <amount>')
        .action((sourceChain, messageId, sourceAddress, tokenSymbol, amount, options) => {
            mainProcessor(receiveToken, options, [sourceChain, messageId, sourceAddress, tokenSymbol, amount], processCommand);
        });

    const deployTokenProgram = new Command()
        .name('deploy-token')
        .description('Deploy token')
        .command('deploy-token <symbol> <name> <decimals>')
        .addOption(new Option('--skip-register', 'Skip register', false))
        .addOption(new Option('--skip-mint', 'Skip mint', false))
        .action((symbol, name, decimals, options) => {
            mainProcessor(deployToken, options, [symbol, name, decimals], processCommand);
        });

    const sendTokenDeploymentProgram = new Command()
        .name('send-deployment')
        .description('Send token deployment')
        .command('send-deployment <symbol> <destination-chain> <destination-address> <fee>')
        .action((symbol, destinationChain, destinationITSAddress, fee, options) => {
            mainProcessor(sendDeployment, options, [symbol, destinationChain, destinationITSAddress, fee], processCommand);
        });

    const receiveTokenDeploymentProgram = new Command()
        .name('receive-deployment')
        .description('Receive token deployment')
        .command('receive-deployment <symbol> <source-chain> <message-id> <source-address> <destination-contract-address> <payload>')
        .action((symbol, sourceChain, messageId, sourceAddress, destinationContractAddress, payload, options) => {
            mainProcessor(
                receiveDeployment,
                options,
                [symbol, sourceChain, messageId, sourceAddress, destinationContractAddress, payload],
                processCommand,
            );
        });

    const setupTrustedAddressProgram = new Command()
        .name('setup-trusted-address')
        .description('Setup trusted address')
        .command('setup-trusted-address <trusted-chain> <trusted-address>')
        .action((trustedChain, trustedAddress, options) => {
            mainProcessor(setupTrustedAddress, options, [trustedChain, trustedAddress], processCommand);
        });

    const mintTokenProgram = new Command()
        .name('mint-token')
        .description('Mint token')
        .command('mint-token <feeAmount> <payload>')
        .action((feeAmount, payload, options) => {
            mainProcessor(mintToken, options, [feeAmount, payload], processCommand);
        });

    const printDeploymentPayloadProgram = new Command()
        .name('print-deployment-info')
        .description('Print deployment info')
        .command('print-deployment-info <name> <symbol> <decimals>')
        .addOption(new Option('--distributor <distributor>', 'Distributor address').default(ethers.constants.HashZero))
        .addOption(new Option('--tokenId <tokenId>', 'Token ID').default(hexlify(randomBytes(32))))
        .action((name, symbol, decimals, options) => {
            const config = loadConfig(options.env);
            printDeploymentInfo(config.sui.contracts, [name, symbol, decimals], options);
        });

    program.addCommand(sendTokenTransferProgram);
    program.addCommand(receiveTokenTransferProgram);
    program.addCommand(deployTokenProgram);
    program.addCommand(sendTokenDeploymentProgram);
    program.addCommand(receiveTokenDeploymentProgram);
    program.addCommand(setupTrustedAddressProgram);
    program.addCommand(mintTokenProgram);
    program.addCommand(printDeploymentPayloadProgram);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
