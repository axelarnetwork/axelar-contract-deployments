const { saveConfig, printInfo } = require('../evm/utils');
const { Command } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { loadSuiConfig } = require('./utils');
const { ethers } = require('hardhat');
const {
    utils: { arrayify },
} = ethers;

const { addBaseOptions, addOptionsToCommands } = require('./cli-utils');
const { getUnitAmount } = require('./amount-utils.js');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');

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
    const [testConfig, , axelarGatewayConfig] = contracts;

    const [sourceChain, messageId, sourceAddress, payload] = args;

    const singletonObjectId = testConfig.objects.singleton;
    const gatewayObjectId = axelarGatewayConfig.objects.gateway;
    const channelId = testConfig.objects.channelId;

    const tx = new TransactionBlock();

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

    tx.moveCall({
        target: `${testConfig.address}::test::execute`,
        arguments: [approvedMessage, tx.object(singletonObjectId)],
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
