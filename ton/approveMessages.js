const { Command } = require('commander');
const { Address, Cell, internal } = require('@ton/ton');
const { getTonClient, loadWallet, sendTransactionWithCost, waitForTransaction, bocToCell, GATEWAY_ADDRESS } = require('./common');
const { APPROVE_MESSAGES_COST } = require('@commonprefix/axelar-cgp-ton');

async function run(encodedPayload) {
    try {
        const client = getTonClient();
        const { contract, key } = await loadWallet(client);

        const gateway = Address.parse(GATEWAY_ADDRESS);
        const approveMessagesCell = bocToCell(encodedPayload);

        var { transfer, seqno } = await sendTransactionWithCost(contract, key, gateway, approveMessagesCell, APPROVE_MESSAGES_COST);

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
