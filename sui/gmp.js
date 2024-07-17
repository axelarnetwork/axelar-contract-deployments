const { saveConfig, printInfo } = require('../evm/utils');
const { Command } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { approvedMessageStruct, singletonStruct } = require('./types-utils');
const { bcs } = require('@mysten/sui.js/bcs');
const { loadSuiConfig, getBcsBytesByObjectId } = require('./utils');
const { ethers } = require('hardhat');
const {
    utils: { arrayify },
} = ethers;

const { addBaseOptions, addOptionsToCommands } = require('./cli-utils');
const { getUnitAmount } = require('./amount-utils.js');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');

// Parse bcs bytes from singleton object to get channel id
async function getChannelId(client, singletonObjectId) {
    const bcsBytes = await getBcsBytesByObjectId(client, singletonObjectId);
    const data = singletonStruct.parse(bcsBytes);
    return '0x' + data.channel.id;
}

async function sendCommand(keypair, client, contracts, args, options) {
    const [destinationChain, destinationAddress, feeAmount, payload] = args;
    const params = options.params;

    const [testConfig, gasServiceConfig] = contracts;
    const gasServicePackageId = gasServiceConfig.address;
    const singletonObjectId = testConfig.objects.singleton;

    const unitAmount = getUnitAmount(feeAmount);
    const walletAddress = keypair.toSuiAddress();
    const refundAddress = options.refundAddress || walletAddress;

    const channelId = await getChannelId(client, singletonObjectId);

    const tx = new TransactionBlock();
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

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Call sent', receipt.digest);
}

async function execute(keypair, client, contracts, args, options) {
    const [testConfig] = contracts;

    const [sourceChain, messageId, sourceAddress, payload] = args;

    const singletonObjectId = testConfig.objects.singleton;
    const channelId = await getChannelId(client, singletonObjectId);

    const encodedMessage = approvedMessageStruct
        .serialize({
            source_chain: sourceChain,
            message_id: messageId,
            source_address: sourceAddress,
            destination_id: channelId,
            payload: arrayify(payload),
        })
        .toBytes();

    const tx = new TransactionBlock();
    tx.moveCall({
        target: `${testConfig.address}::test::execute`,
        arguments: [tx.pure(bcs.vector(bcs.u8()).serialize(encodedMessage).toBytes()), tx.object(singletonObjectId)],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Call executed', receipt.digest);
}

async function processCommand(command, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    const contracts = [chain.contracts.test, chain.contracts.GasService];

    await command(keypair, client, contracts, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadSuiConfig(options.env);
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
        .command('execute <sourceChain> <messageId> <sourceAddress> <payload>')
        .action((sourceChain, messageId, sourceAddress, payload, options) => {
            mainProcessor(execute, options, [sourceChain, messageId, sourceAddress, payload], processCommand);
        });

    program.addCommand(sendCallProgram);
    program.addCommand(executeCommand);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
