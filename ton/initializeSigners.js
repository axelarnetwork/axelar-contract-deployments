const { Command } = require('commander');
const { Address, internal } = require('@ton/ton');
const {
    getTonClient,
    loadWallet,
    waitForTransaction,
    parseWeightedSigners,
    sendTransactionWithCost,
    GATEWAY_ADDRESS,
} = require('./common');
const { INIT_SIGNERS_COST, serializeWeightedSigners, buildInitializeSignersMessage } = require('@commonprefix/axelar-cgp-ton');

function createInitializeSignersCell(weightedSigners) {
    const signersCell = serializeWeightedSigners(weightedSigners);
    return buildInitializeSignersMessage(signersCell);
}

async function run(weightedSignersJson) {
    try {
        console.log('Parsing weighted signers...');
        const weightedSigners = parseWeightedSigners(weightedSignersJson);

        console.log(`Parsed ${weightedSigners.signers.length} signers with threshold ${weightedSigners.threshold}`);

        const client = getTonClient();
        const { contract, key } = await loadWallet(client);

        const gateway = Address.parse(GATEWAY_ADDRESS);
        const initializeSignersCell = createInitializeSignersCell(weightedSigners);

        var { transfer, seqno } = await sendTransactionWithCost(contract, key, gateway, initializeSignersCell, INIT_SIGNERS_COST);

        console.log('Initialize Signers transaction sent successfully!');

        await waitForTransaction(contract, seqno);
    } catch (error) {
        console.error('Error in initialize signers:', error);
        throw error;
    }
}

// Set up command line interface
if (require.main === module) {
    const program = new Command();
    program
        .name('initialiseSigners')
        .description('Initialize signers on TON gateway')
        .argument('<weightedSignersJson>', 'WeightedSigners JSON string')
        .action(run);

    program.parse();
}
