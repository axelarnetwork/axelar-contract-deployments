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

async function sendCommand(chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    const [destinationChain, destinationAddress, payload] = args;

    const testConfig = chain.contracts.test;
    const singletonObjectId = testConfig.objects.singleton;

    const tx = new TransactionBlock();
    tx.moveCall({
        target: `${chain.contracts.test.address}::test::send_call`,
        arguments: [
            tx.object(singletonObjectId),
            tx.pure(bcs.string().serialize(destinationChain).toBytes()),
            tx.pure(bcs.string().serialize(destinationAddress).toBytes()),
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()),
        ],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Call sent', receipt.digest);
}

async function processCommand(command, chain, args, options) {
    switch (command) {
        case 'send-call':
            printInfo('Action', 'Sending call');
            return sendCommand(chain, args, options);
        default:
            throw new Error(`Unknown command: ${command}`);
    }
}

async function mainProcessor(command, options, args, processor) {
    const config = loadSuiConfig(options.env);
    await processor(command, config.sui, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('gmp').description('Example of SUI gmp commands');

    const sendCallProgram = program
        .name('send-call')
        .description('Example of SUI contract call')
        .command('send-call <destChain> <destContractAddress> <payload>');

    addBaseOptions(sendCallProgram);

    sendCallProgram.action((destChain, destContractAddress, payload, options) => {
        mainProcessor('send-call', options, [destChain, destContractAddress, payload], processCommand);
    });

    program.addCommand(sendCallProgram);
    program.parse();
}
