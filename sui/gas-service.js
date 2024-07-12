const { saveConfig, prompt, printInfo } = require('../evm/utils');
const { Command, Option } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { loadSuiConfig } = require('./utils');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, keccak256, toUtf8Bytes },
    constants: { HashZero },
} = ethers;

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');

async function payGas(config, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);
    const walletAddress = keypair.toSuiAddress();

    const gasServiceConfig = chain.contracts.axelar_gas_service;
    const gasServicePackageId = gasServiceConfig.address;

    const gatewayConfig = chain.contracts.axelar_gateway;
    const gatewayPackageId = gatewayConfig.address;

    const tx = new TransactionBlock();

    const [coin] = tx.splitCoins(tx.gas, [100]);

    const [destinationChain, destinationAddress, payload] = args;

    let channel = options.channel;

    if (!options.channel) {
        [channel] = tx.moveCall({
            target: `${gatewayPackageId}::channel::new`,
            arguments: [],
        });
    }

    tx.moveCall({
        target: `${gasServicePackageId}::gas_service::pay_gas`,
        arguments: [
            gasServicePackageId, // Gas service package ID
            coin, // Coin<SUI>
            channel, // Channel address
            tx.pure(bcs.string().serialize(destinationChain).toBytes()), // Destination chain
            tx.pure(bcs.string().serialize(destinationAddress).toBytes()), // Destination address
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()), // Payload
            walletAddress, // Refund address
            bcs.vector(), // Params
        ],
    });
}

async function processCommand(command, config, chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts.axelar_gas_service) {
        throw new Error('Axelar gas service contract not found');
    }

    switch (command) {
        case 'pay_gas':
            await payGas(config, chain, args, options);
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

    const payGasProgram = program.command('pay_gas <destination_chain> <destination_address> <payload>');
    payGasProgram.description('Pay gas to the destination chain.');
    payGasProgram.action((destinationChain, destinationAddress, payload, options) => {
        mainProcessor('pay_gas', options, [destinationChain, destinationAddress, payload], processCommand);
    });

    program.addCommand(payGasProgram);

    addBaseOptions(program);

    program.parse();
}
