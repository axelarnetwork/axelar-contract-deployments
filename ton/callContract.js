const { Command } = require('commander');
const { Address, internal, beginCell } = require('@ton/ton');
const { getTonClient, loadWallet, bufferToCell, waitForTransaction, GATEWAY_ADDRESS } = require('./common');

// Constants
const CALL_CONTRACT_COST = '0.3';
const OP_CALL_CONTRACT = 0x00000009;

function buildCallContractMessageBody(
    destinationChainString,
    destinationContractAddressString,
    payloadBuffer,
) {
    const destinationChain = beginCell().storeStringTail(destinationChainString).endCell();
    const destinationContractAddress = beginCell().storeStringTail(destinationContractAddressString).endCell();
    const payload = bufferToCell(payloadBuffer);

    return beginCell()
        .storeUint(OP_CALL_CONTRACT, 32)
        .storeRef(destinationChain)
        .storeRef(destinationContractAddress)
        .storeRef(payload)
        .endCell();
}

async function run(destinationChain, destinationContractAddress, payload) {
    try {
        const client = getTonClient();
        const { contract, key, wallet } = await loadWallet(client);
        const gateway = Address.parse(GATEWAY_ADDRESS);

        const payloadBuffer = Buffer.from(payload, 'hex');

        const callContractCell = buildCallContractMessageBody(
            destinationChain,
            destinationContractAddress,
            payloadBuffer
        );

        const message = internal({
            to: gateway,
            value: CALL_CONTRACT_COST,
            body: callContractCell,
        });

        const seqno = await contract.getSeqno();
        console.log('Current wallet seqno:', seqno);

        console.log('Sending call contract transaction...');
        const transfer = await contract.sendTransfer({
            secretKey: key.secretKey,
            messages: [message],
            seqno: seqno,
            amount: CALL_CONTRACT_COST,
        });

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
