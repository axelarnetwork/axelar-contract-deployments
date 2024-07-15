const { saveConfig, printInfo } = require('../evm/utils');
const { Command } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { approvedMessageStruct } = require('./types-utils');
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

async function execute(chain, args, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    const [sourceChain, messageId, sourceAddress, destinationId, payload] = args;

    const encodedMessage = approvedMessageStruct
        .serialize({
            source_chain: sourceChain,
            message_id: messageId,
            source_address: sourceAddress,
            destination_id: destinationId,
            payload,
        })
        .toBytes();

    const testConfig = chain.contracts.test;
    const singletonObjectId = testConfig.objects.singleton;

    const tx = new TransactionBlock();
    tx.moveCall({
        target: `${chain.contracts.test.address}::test::execute`,
        arguments: [tx.pure(bcs.vector(bcs.u8()).serialize(encodedMessage).toBytes()), tx.object(singletonObjectId)],
    });

    const receipt = await broadcast(client, keypair, tx);

    printInfo('Call executed', receipt.digest);
}

async function processCommand(command, chain, args, options) {
    switch (command) {
        case 'send-call':
            printInfo('Action', 'Send Call');
            return sendCommand(chain, args, options);
        case 'execute':
            printInfo('Action', 'Execute');
            return execute(chain, args, options);
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

    const sendCallProgram = new Command()
        .name('send-call')
        .description('Send gmp contract call')
        .command('send-call <destChain> <destContractAddress> <payload>');

    const executeCommand = new Command()
        .name('execute')
        .description('Execute gmp contract call')
        .command('execute <sourceChain> <messageId> <sourceAddress> <destinationId> <payload>');

    addBaseOptions(sendCallProgram);
    addBaseOptions(executeCommand);

    sendCallProgram.action((destChain, destContractAddress, payload, options) => {
        mainProcessor('send-call', options, [destChain, destContractAddress, payload], processCommand);
    });

    executeCommand.action((sourceChain, messageId, sourceAddress, destinationId, payload, options) => {
        mainProcessor('execute', options, [sourceChain, messageId, sourceAddress, destinationId, payload], processCommand);
    });

    program.addCommand(sendCallProgram);
    program.addCommand(executeCommand);
    program.parse();
}
