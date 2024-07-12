const { saveConfig, printInfo } = require('../evm/utils');
const { Command } = require('commander');
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

    const tx = new TransactionBlock();

    const [senderAddress, amount, destinationChain, destinationAddress, payload] = args;

    const atomicAmount = ethers.utils.parseUnits(amount, 6).toString();

    const [coin] = tx.splitCoins(tx.gas, [atomicAmount]);

    tx.moveCall({
        target: `${gasServicePackageId}::gas_service::pay_gas`,
        arguments: [
            tx.object(gasServiceConfig.objects.gas_service),
            coin, // Coin<SUI>
            tx.pure.address(senderAddress), // Channel address
            tx.pure(bcs.string().serialize(destinationChain).toBytes()), // Destination chain
            tx.pure(bcs.string().serialize(destinationAddress).toBytes()), // Destination address
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()), // Payload
            tx.pure.address(walletAddress), // Refund address
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify('0x')).toBytes()), // Params
        ],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Gas paid', receipt.digest);
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

    const payGasProgram = program.command('pay_gas <sender_address> <amount> <destination_chain> <destination_address> <payload>');
    payGasProgram.description('Pay gas to the destination chain.');
    payGasProgram.action((senderAddress, amount, destinationChain, destinationAddress, payload, options) => {
        mainProcessor('pay_gas', options, [senderAddress, amount, destinationChain, destinationAddress, payload], processCommand);
    });


    program.addCommand(payGasProgram);

    addBaseOptions(payGasProgram);

    program.parse();
}
