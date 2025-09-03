const { Command } = require('commander');
const { Address, Cell, internal } = require('@ton/ton');
const {
    getTonClient,
    loadWallet,
    waitForTransaction,
    parseWeightedSigners,
    sendTransactionWithCost,
    GATEWAY_ADDRESS,
} = require('./common');
const {
    START_SIGNER_ROTATION_COST,
    ROTATE_SIGNERS_COST,
    buildRotateSignersMessage,
    serializeWeightedSigners,
} = require('@commonprefix/axelar-cgp-ton');

function createStartSignerRotationCell(encodedPayload) {
    return Cell.fromBoc(Buffer.from(encodedPayload, 'hex'))[0];
}

function createRotateSignersCell(weightedSigners) {
    const signersCell = serializeWeightedSigners(weightedSigners);
    return buildRotateSignersMessage(signersCell);
}

async function run(encodedPayload, weightedSignersJson) {
    try {
        const client = getTonClient();
        const { contract, key } = await loadWallet(client);

        console.log('Parsing weighted signers...');
        const weightedSigners = parseWeightedSigners(weightedSignersJson);

        console.log(`Parsed ${weightedSigners.signers.length} signers with threshold ${weightedSigners.threshold}`);

        const gateway = Address.parse(GATEWAY_ADDRESS);
        const startSignerRotationCell = createStartSignerRotationCell(encodedPayload);

        var { transfer, seqno } = await sendTransactionWithCost(
            contract,
            key,
            gateway,
            startSignerRotationCell,
            START_SIGNER_ROTATION_COST,
        );

        console.log('StartSignerRotation transaction sent successfully!');
        await waitForTransaction(contract, seqno);

        const rotateSignersCell = createRotateSignersCell(weightedSigners);

        var { transfer, seqno } = await sendTransactionWithCost(contract, key, gateway, rotateSignersCell, ROTATE_SIGNERS_COST);

        console.log('Rotate Signers transaction sent successfully!');

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
        .name('signerRotation')
        .description('Rotate signers on TON gateway')
        .argument('<encodedPayload>', 'Encoded payload in hex format')
        .argument('<weightedSignersJson>', 'WeightedSigners JSON string')
        .action(run);

    program.parse();
}
