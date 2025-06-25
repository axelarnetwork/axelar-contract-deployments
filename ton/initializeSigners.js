const { Command } = require('commander');
const { Address, internal, beginCell, Dictionary } = require('@ton/ton');
const { getTonClient, loadWallet, waitForTransaction, GATEWAY_ADDRESS } = require('./common');

// Constants
const INITIALIZE_SIGNERS_COST = '0.15';
const OP_INITIALIZE_SIGNERS = 0x00000012;

function combineData(signer, weight, signature) {
    const sigBuffer = Buffer.alloc(64, 0)

    const signerBuffer = Buffer.alloc(32);
    const weightBuffer = Buffer.alloc(16);

    const signerHex = signer.toString(16).padStart(64, '0');
    const weightHex = weight.toString(16).padStart(32, '0');

    Buffer.from(signerHex, 'hex').copy(signerBuffer);
    Buffer.from(weightHex, 'hex').copy(weightBuffer);

    return Buffer.concat([signerBuffer, weightBuffer, sigBuffer]);
}

function serializeWeightedSigner(signer) {
    return combineData(signer.signer, signer.weight, signer.signature);
}

function serializeWeightedSigners(data) {
    const signersDict = Dictionary.empty(
        Dictionary.Keys.Uint(16),
        Dictionary.Values.Buffer(112) // (256 + 128 + 512) / 8 = 112 bytes
    );

    data.signers.forEach((signer, index) => {
        signersDict.set(index, serializeWeightedSigner(signer));
    });

    return beginCell()
        .storeDict(signersDict)
        .storeUint(data.threshold, 128)
        .storeUint(data.nonce, 256)
        .endCell();
}

function buildInitializeSigners(signers) {
    return beginCell()
        .storeUint(OP_INITIALIZE_SIGNERS, 32)
        .storeRef(signers)
        .endCell();
}

function getEmptySignature() {
    const zeroBuffer = Buffer.alloc(64, 0);
    const zeroBase64 = zeroBuffer.toString('base64');
    return zeroBase64;
}

function parseWeightedSigners(jsonString) {
    try {
        const parsed = JSON.parse(jsonString);

        // Validate structure
        if (!parsed.signers || !Array.isArray(parsed.signers)) {
            throw new Error('Invalid format: signers must be an array');
        }

        if (typeof parsed.threshold === 'undefined' || typeof parsed.nonce === 'undefined') {
            throw new Error('Invalid format: threshold and nonce are required');
        }

        // Convert to proper types
        const weightedSigners = {
            signers: parsed.signers.map(signer => ({
                signer: BigInt(signer.signer),
                weight: BigInt(signer.weight),
                signature: getEmptySignature()
            })),
            threshold: BigInt(parsed.threshold),
            nonce: BigInt(parsed.nonce)
        };


        return weightedSigners;
    } catch (error) {
        if (error instanceof SyntaxError) {
            throw new Error('Invalid JSON format');
        }
        throw error;
    }
}

function createInitializeSignersCell(weightedSigners) {
    const signersCell = serializeWeightedSigners(weightedSigners);
    return buildInitializeSigners(signersCell);
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

        const message = internal({
            to: gateway,
            value: INITIALIZE_SIGNERS_COST,
            body: initializeSignersCell,
        });

        const seqno = await contract.getSeqno();
        console.log('Current wallet seqno:', seqno);

        console.log('Sending initialize signers transaction...');
        const transfer = await contract.sendTransfer({
            secretKey: key.secretKey,
            messages: [message],
            seqno: seqno,
            amount: INITIALIZE_SIGNERS_COST,
        });

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

module.exports = {
    serializeWeightedSigners,
    serializeWeightedSigner,
    buildInitializeSigners,
    parseWeightedSigners,
    createInitializeSignersCell
};
