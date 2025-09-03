const { Command } = require('commander');
const { Address, internal, toNano } = require('@ton/ton');
const { getTonClient, loadWallet, waitForTransaction, sendTransactionWithCost, GATEWAY_ADDRESS } = require('./common');
const { buildContractCallMessageChained } = require('@commonprefix/axelar-cgp-ton');

const CALL_CONTRACT_COST = toNano('0.1');

async function run(destinationChain, destinationContractAddress, payload) {
    try {
        const client = getTonClient();
        const { contract, key, wallet } = await loadWallet(client);
        const gateway = Address.parse(GATEWAY_ADDRESS);

        const payloadBuffer = Buffer.from(payload, 'hex');
        const callContractCell = buildContractCallMessageChained(destinationChain, destinationContractAddress, payloadBuffer);

        var { transfer, seqno } = await sendTransactionWithCost(contract, key, gateway, callContractCell, CALL_CONTRACT_COST);

        console.log('Call contract transaction sent successfully!');

        await waitForTransaction(contract, seqno);
    } catch (error) {
        console.error('Error in call contract:', error);
        throw error;
    }
}

// Set up command line interface
if (require.main === module) {
    const program = new Command();
    program
        .name('callContract')
        .description('Call contract on TON gateway')
        .argument('<destinationChain>', 'Destination chain name (e.g. avalanche-fuji)')
        .argument('<destinationContractAddress>', 'Destination contract address (e.g. 0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC)')
        .argument('<payload>', 'Payload in hex (e.g. 48656c6c6f2066726f6d204176616c616e63686521)')
        .action(run);

    program.parse();
}
