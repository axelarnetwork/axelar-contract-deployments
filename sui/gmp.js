const { Command } = require('commander');
const { Transaction } = require('@mysten/sui/transactions');
const { bcs } = require('@mysten/sui/bcs');
const { loadConfig, saveConfig } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    getUnitAmount,
    getWallet,
    printWalletInfo,
    broadcast,
    parseGatewayInfo,
    parseDiscoveryInfo,
    broadcastExecuteApprovedMessage,
} = require('./utils');
const { ethers } = require('hardhat');
const {
    utils: { arrayify },
} = ethers;

async function sendCommand(keypair, client, chain, args, options) {
    const [destinationChain, destinationAddress, feeAmount, payload] = args;
    const params = options.params;
    const gasServiceObjectId = chain.contracts.GasService.objects.GasService;
    const gatewayObjectId = chain.contracts.AxelarGateway.objects.Gateway;
    const singletonObjectId = chain.contracts.Example.objects.GmpSingleton;

    const unitAmount = getUnitAmount(feeAmount);
    const walletAddress = keypair.toSuiAddress();
    const refundAddress = options.refundAddress || walletAddress;

    const tx = new Transaction();
    const [coin] = tx.splitCoins(tx.gas, [unitAmount]);

    tx.moveCall({
        target: `${chain.contracts.Example.address}::gmp::send_call`,
        arguments: [
            tx.object(singletonObjectId),
            tx.object(gatewayObjectId),
            tx.object(gasServiceObjectId),
            tx.pure(bcs.string().serialize(destinationChain).toBytes()),
            tx.pure(bcs.string().serialize(destinationAddress).toBytes()),
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()),
            tx.pure.address(refundAddress),
            coin,
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(params)).toBytes()),
        ],
    });

    await broadcast(client, keypair, tx, 'Call Sent');
}

async function execute(keypair, client, chain, args, options) {
    const [sourceChain, messageId, sourceAddress, payload] = args;

    const { Example } = chain.contracts;

    const channelId = options.channelId || chain.contracts.Example.objects.GmpChannelId;

    if (!channelId) {
        throw new Error('Please provide either a channel id (--channelId) or deploy the Example contract');
    }

    const gatewayInfo = parseGatewayInfo(chain.contracts);
    const discoveryInfo = parseDiscoveryInfo(chain.contracts);
    const messageInfo = {
        source_chain: sourceChain,
        message_id: messageId,
        source_address: sourceAddress,
        destination_id: Example.objects.GmpChannelId,
        payload,
    };

    await broadcastExecuteApprovedMessage(client, keypair, discoveryInfo, gatewayInfo, messageInfo, 'Call Executed');
}

async function processCommand(command, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    await command(keypair, client, chain, args, options);
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
