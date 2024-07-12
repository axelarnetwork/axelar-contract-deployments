const { saveConfig, printInfo } = require('../evm/utils');
const { Command, Option } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { loadSuiConfig } = require('./utils');
const { ethers } = require('hardhat');
const {
    utils: { arrayify },
} = ethers;

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');

async function payGas(config, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);
    const walletAddress = keypair.toSuiAddress();

    const gasServiceConfig = chain.contracts.axelar_gas_service;
    const gasServicePackageId = gasServiceConfig.address;

    const channel = options.channel;

    const refundAddress = options.refund_address || walletAddress;
    const params = options.params || '0x';

    const tx = new TransactionBlock();

    const [amount, destinationChain, destinationAddress, payload] = args;

    const atomicAmount = ethers.utils.parseUnits(amount, 6).toString();

    const [coin] = tx.splitCoins(tx.gas, [atomicAmount]);

    tx.moveCall({
        target: `${gasServicePackageId}::gas_service::pay_gas`,
        arguments: [
            tx.object(gasServiceConfig.objects.gas_service),
            coin, // Coin<SUI>
            tx.pure.address(channel), // Channel address
            tx.pure(bcs.string().serialize(destinationChain).toBytes()), // Destination chain
            tx.pure(bcs.string().serialize(destinationAddress).toBytes()), // Destination address
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()), // Payload
            tx.pure.address(refundAddress), // Refund address
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(params)).toBytes()), // Params
        ],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas paid', receipt.digest);
}

async function addGas(config, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);
    const walletAddress = keypair.toSuiAddress();

    const gasServiceConfig = chain.contracts.axelar_gas_service;
    const gasServicePackageId = gasServiceConfig.address;

    const refundAddress = options.refund_address || walletAddress;
    const params = options.params || '0x';

    const tx = new TransactionBlock();

    const [messageId, amount] = args;

    const atomicAmount = ethers.utils.parseUnits(amount, 6).toString();

    const [coin] = tx.splitCoins(tx.gas, [atomicAmount]);

    tx.moveCall({
        target: `${gasServicePackageId}::gas_service::add_gas`,
        arguments: [
            tx.object(gasServiceConfig.objects.gas_service),
            coin, // Coin<SUI>
            tx.pure(bcs.string().serialize(messageId).toBytes()), // Message ID for the contract call
            tx.pure.address(refundAddress), // Refund address
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(params)).toBytes()), // Params
        ],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas added', receipt.digest);
}

async function processCommand(command, config, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts.axelar_gas_service) {
        throw new Error('Axelar gas service contract not found');
    }

    switch (command) {
        case 'pay_gas':
            printInfo('Action', 'Pay gas');
            await payGas(config, chain, args, options);
            break;
        case 'add_gas':
            printInfo('Action', 'Add gas');
            await addGas(config, chain, args, options);
            break;
    }
}

async function mainProcessor(command, options, args, processor) {
    const config = loadSuiConfig(options.env);
    await processor(command, config, config.sui, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('gas-service').description('Interact with the gas service contract.');

    const payGasProgram = program
        .command('pay_gas <amount> <destination_chain> <destination_address> <payload>')
        .description('Pay gas for the contract call.')
        .requiredOption('--channel <channel>', 'Existing channel ID to initiate a cross-chain message over')
        .option('--refund_address <refundAddress>', 'Refund address. Default is the sender address.')
        .option('--params <params>', 'Params. Default is empty.')
        .action((amount, destinationChain, destinationAddress, payload, options) => {
            mainProcessor('pay_gas', options, [amount, destinationChain, destinationAddress, payload], processCommand);
        });

    const addGasProgram = program
        .command('add_gas <message_id> <amount>')
        .description('Add gas for the contract call.')
        .option('--refund_address <refundAddress>', 'Refund address.')
        .option('--params <params>', 'Params. Default is empty.')
        .action((messageId, amount, options) => {
            mainProcessor('add_gas', options, [messageId, amount], processCommand);
        });

    program.addCommand(payGasProgram);
    program.addCommand(addGasProgram);

    addBaseOptions(payGasProgram);
    addBaseOptions(addGasProgram);

    program.parse();
}
