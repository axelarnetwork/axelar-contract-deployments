const { Command } = require('commander');
const { Transaction } = require('@mysten/sui/transactions');
const { bcs } = require('@mysten/sui/bcs');
const { saveConfig, printInfo } = require('../common/utils');
const {
  loadConfig,
  getBcsBytesByObjectId,
  addBaseOptions,
    addOptionsToCommands,
    getUnitAmount,
    getWallet,
    printWalletInfo,
    discoveryStruct,
    broadcast,
} = require('./utils');
const { ethers } = require('hardhat');
const {
    utils: { arrayify },
} = ethers;

async function sendCommand(keypair, client, contracts, args, options) {
    const [destinationChain, destinationAddress, feeAmount, payload] = args;
    const params = options.params;

    const [testConfig, gasServiceConfig] = contracts;
    const gasServicePackageId = gasServiceConfig.address;
    const singletonObjectId = testConfig.objects.singleton;
    const channelId = testConfig.objects.channelId;

    const unitAmount = getUnitAmount(feeAmount);
    const walletAddress = keypair.toSuiAddress();
    const refundAddress = options.refundAddress || walletAddress;

    const tx = new Transaction();
    const [coin] = tx.splitCoins(tx.gas, [unitAmount]);

    tx.moveCall({
        target: `${testConfig.address}::test::send_call`,
        arguments: [
            tx.object(singletonObjectId),
            tx.pure(bcs.string().serialize(destinationChain).toBytes()),
            tx.pure(bcs.string().serialize(destinationAddress).toBytes()),
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()),
        ],
    });

    if (gasServiceConfig) {
        tx.moveCall({
            target: `${gasServicePackageId}::gas_service::pay_gas`,
            arguments: [
                tx.object(gasServiceConfig.objects.GasService),
                coin, // Coin<SUI>
                tx.pure.address(channelId), // Channel address
                tx.pure(bcs.string().serialize(destinationChain).toBytes()), // Destination chain
                tx.pure(bcs.string().serialize(destinationAddress).toBytes()), // Destination address
                tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()), // Payload
                tx.pure.address(refundAddress), // Refund address
                tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(params)).toBytes()), // Params
            ],
        });
    }

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Call sent', receipt.digest);
}

async function execute(keypair, client, contracts, args, options) {
    const [testConfig, , axelarGatewayConfig] = contracts;

    const [sourceChain, messageId, sourceAddress, payload] = args;

    const gatewayObjectId = axelarGatewayConfig.objects.Gateway;
    const discoveryObjectId = axelarGatewayConfig.objects.RelayerDiscovery;

    // Get the channel id from the options or use the channel id from the deployed test contract object.
    const channelId = options.channelId || testConfig.objects.channelId;

    if (!channelId) {
        throw new Error('Please provide either a channel id (--channelId) or deploy the test contract');
    }

    // Get Discovery table id from discovery object
    const tableBcsBytes = await getBcsBytesByObjectId(client, discoveryObjectId);
    const { fields } = discoveryStruct.parse(tableBcsBytes);
    const tableId = fields.id;

    // Get the transaction list from the discovery table
    const tableResult = await client.getDynamicFields({
        parentId: tableId,
    });
    const transactionList = tableResult.data;

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

    const tx = new Transaction();

    // Take the approved message from the gateway contract.
    // Note: The message needed to be approved first.
    const approvedMessage = tx.moveCall({
        target: `${axelarGatewayConfig.address}::gateway::take_approved_message`,
        arguments: [
            tx.object(gatewayObjectId),
            tx.pure(bcs.string().serialize(sourceChain).toBytes()),
            tx.pure(bcs.string().serialize(messageId).toBytes()),
            tx.pure(bcs.string().serialize(sourceAddress).toBytes()),
            tx.pure.address(channelId),
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()),
        ],
    });

    const { module_name: moduleName, name, package_id: packageId } = txFields.function.fields;

    // Build the arguments for the move call
    // There're 5 types of arguments as mentioned in the following link https://github.com/axelarnetwork/axelar-cgp-sui/blob/72579e5c7735da61d215bd712627edad562cb82a/src/bcs.ts#L44-L49
    const txArgs = txFields.arguments.map(([argType, ...arg]) => {
        if (argType === 0) {
            return tx.object(Buffer.from(arg).toString('hex'));
        } else if (argType === 1) {
            // TODO: handle pures followed by the bcs encoded form of the pure
            throw new Error('Not implemented yet');
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
    tx.moveCall({
        target: `${packageId}::${moduleName}::${name}`,
        arguments: txArgs,
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Call executed', receipt.digest);
}

async function processCommand(command, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    const contracts = [chain.contracts.test, chain.contracts.GasService, chain.contracts.axelar_gateway];

    await command(keypair, client, contracts, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    await processor(command, config.sui, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('gmp').description('Example of SUI gmp commands');

    const sendCallProgram = new Command()
        .name('sendCall')
        .description('Send gmp contract call')
        .command('sendCall <destChain> <destContractAddress> <feeAmount> <payload>')
        .option('--params <params>', 'GMP call params. Default is empty.', '0x')
        .action((destChain, destContractAddress, feeAmount, payload, options) => {
            mainProcessor(sendCommand, options, [destChain, destContractAddress, feeAmount, payload], processCommand);
        });

    const executeCommand = new Command()
        .name('execute')
        .description('Execute gmp contract call')
        .option('--channelId <channelId>', 'Channel id for the destination contract')
        .command('execute <sourceChain> <messageId> <sourceAddress> <payload>')
        .action((sourceChain, messageId, sourceAddress, payload, options) => {
            mainProcessor(execute, options, [sourceChain, messageId, sourceAddress, payload], processCommand);
        });

    program.addCommand(sendCallProgram);
    program.addCommand(executeCommand);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
