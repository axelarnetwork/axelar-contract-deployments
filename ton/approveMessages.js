const { Command } = require('commander');
const { Address, Cell, internal } = require('@ton/ton');
const { getTonClient, loadWallet, waitForTransaction, GATEWAY_ADDRESS } = require('./common');

// Constants
const APPROVE_MESSAGES_COST = '2';

function createApproveMessagesCell(encodedPayload) {
    return Cell.fromBoc(Buffer.from(encodedPayload, 'hex'))[0];
}

async function run(encodedPayload) {
    try {
        const client = getTonClient();
        const { contract, key } = await loadWallet(client);

        const gateway = Address.parse(GATEWAY_ADDRESS);
        const approveMessagesCell = createApproveMessagesCell(encodedPayload);

        const message = internal({
            to: gateway,
            value: APPROVE_MESSAGES_COST,
            body: approveMessagesCell,
        });

        const seqno = await contract.getSeqno();
        console.log('Current wallet seqno:', seqno);

        console.log('Sending approve messages transaction...');
        const transfer = await contract.sendTransfer({
            secretKey: key.secretKey,
            messages: [message],
            seqno: seqno,
            amount: APPROVE_MESSAGES_COST,
        });

        console.log('Approve messages transaction sent successfully!');

        await waitForTransaction(contract, seqno);

    } catch (error) {
        console.error('Error in approve messages:', error);
        throw error;
    }
}

// Set up command line interface
if (require.main === module) {
    const program = new Command();
    program
        .name('approveMessages')
        .description('Approve messages on TON gateway')
        .argument('<encodedPayload>', 'Encoded payload in hex format')
        .action(run);

    program.parse();
}
