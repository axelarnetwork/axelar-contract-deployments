const { Command } = require('commander');
const {
    bcsStructs,
    ITSMessageType,
    SUI_PACKAGE_ID,
    CLOCK_PACKAGE_ID,
    TxBuilder,
    copyMovePackage,
} = require('@axelar-network/axelar-cgp-sui');
const { loadConfig, saveConfig, printInfo } = require('../common/utils');
const {
    getBcsBytesByObjectId,
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
    utils: { defaultAbiCoder, keccak256 },
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
    const { Example, RelayerDiscovery, AxelarGateway, ITS } = contracts;

    const [sourceChain, messageId, sourceAddress, tokenSymbol, amount] = args;
    const itsData = options.data || '0x';

    // Prepare Object Ids
    const symbol = tokenSymbol.toUpperCase();
    const discoveryObjectId = RelayerDiscovery.objects.RelayerDiscoveryv0;
    const gatewayObjectId = AxelarGateway.objects.Gateway;
    const itsChannelId = ITS.objects.ChannelId;
    const channelId = options.channelId || Example.objects.ItsChannelId;

    if (!channelId) {
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
        [ITSMessageType.InterchainTokenTransfer, tokenId, sourceAddress, channelId, unitAmount, itsData],
    );

    const payloadHash = keccak256(payload);
    printInfo('Payload Hash', payloadHash);

    // Get Discovery table id from discovery object
    const transactionList = await getTransactionList(client, discoveryObjectId);

    // Find the transaction with associated channel id
    const transaction = transactionList.find((row) => row.name.value === channelId);

    if (!transaction) {
        throw new Error(`Transaction not found for channel ${channelId}`);
    }

    // Get the transaction object from the object id
    const txObject = await client.getObject({
        id: transaction.objectId,
        options: {
            showContent: true,
        },
    });

    // Extract the fields from the transaction object
    const txFields = txObject.data.content.fields.value.fields.move_calls[0].fields;

    const receiveTxBuilder = new TxBuilder(client);

    // Take the approved message from the gateway contract.
    const approvedMessage = await receiveTxBuilder.moveCall({
        target: `${AxelarGateway.address}::gateway::take_approved_message`,
        arguments: [gatewayObjectId, sourceChain, messageId, sourceAddress, itsChannelId, payload],
    });

    const { module_name: moduleName, name, package_id: packageId } = txFields.function.fields;

    // Build the arguments for the move call
    // There're 5 types of arguments as mentioned in the following link https://github.com/axelarnetwork/axelar-cgp-sui/blob/72579e5c7735da61d215bd712627edad562cb82a/src/bcs.ts#L44-L49
    const txArgs = txFields.arguments.map(([argType, ...arg]) => {
        if (argType === 0) {
            return '0x' + Buffer.from(arg).toString('hex');
        } else if (argType === 1) {
            // TODO: handle pures followed by the bcs encoded form of the pure
            // throw new Error('Not implemented yet');
        } else if (argType === 2) {
            return approvedMessage;
        } else if (argType === 3) {
            // TODO: handle the payload of the contract call (to be passed into the intermediate function)
            throw new Error('Not implemented yet');
        } else if (argType === 4) {
            // TODO: handle an argument returned from a previous move call, followed by a u8 specified which call to get the return of (0 for the first transaction AFTER the one that gets ApprovedMessage out), and then another u8 specifying which argument to input.
            throw new Error('Not implemented yet');
        }

        throw new Error(`Unknown argument type: ${argType}`);
    });

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
    const registerTxBuilder = new TxBuilder(client);

    await registerTxBuilder.moveCall({
        target: `${Example.address}::its::register_coin`,
        arguments: [ITS.objects.ITS, Metadata],
        typeArguments: [tokenType],
    });

    const result = await broadcastFromTxBuilder(registerTxBuilder, keypair, `Registered ${symbol} in ITS`, { showEvents: true });

    // Save the deployed token info in the contracts object.
    contracts[symbol.toUpperCase()] = {
        address: packageId,
        typeArgument: tokenType,
        decimals,
        objects: {
            TreasuryCap,
            Metadata,
            TokenId: result.events[0].parsedJson.token_id.id,
        },
    };

    // Mint Token
    const mintTxBuilder = new TxBuilder(client);

    const coin = await mintTxBuilder.moveCall({
        target: `${SUI_PACKAGE_ID}::coin::mint`,
        arguments: [TreasuryCap, getUnitAmount('1000', decimals)],
        typeArguments: [tokenType],
    });

    mintTxBuilder.tx.transferObjects([coin], walletAddress);

    await broadcastFromTxBuilder(mintTxBuilder, keypair, `Minted 1,000 ${symbol}`);
}

async function sendTokenDeployment(keypair, client, contracts, args, options) {}

async function receiveTokenDeployment(keypair, client, contracts, args, options) {}

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
        .action((symbol, name, decimals, options) => {
            mainProcessor(deployToken, options, [symbol, name, decimals], processCommand);
        });

    const sendTokenDeploymentProgram = new Command()
        .name('send-deployment')
        .description('Send token deployment')
        .command('send-deployment  <payload>')
        .action((feeAmount, payload, options) => {
            mainProcessor(sendTokenDeployment, options, [feeAmount, payload], processCommand);
        });

    const receiveTokenDeploymentProgram = new Command()
        .name('receive-deployment')
        .description('Receive token deployment')
        .command('receive-deployment <message-id> <source-address> <payload>')
        .action((messageId, sourceAddress, payload, options) => {
            mainProcessor(receiveTokenDeployment, options, [messageId, sourceAddress, payload], processCommand);
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

    program.addCommand(sendTokenTransferProgram);
    program.addCommand(receiveTokenTransferProgram);
    program.addCommand(deployTokenProgram);
    program.addCommand(sendTokenDeploymentProgram);
    program.addCommand(receiveTokenDeploymentProgram);
    program.addCommand(setupTrustedAddressProgram);
    program.addCommand(mintTokenProgram);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
