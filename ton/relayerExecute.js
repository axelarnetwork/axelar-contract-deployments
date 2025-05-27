const { Command } = require('commander');
const { Address, internal, beginCell } = require('@ton/ton');
const { getTonClient, loadWallet, waitForTransaction, GATEWAY_ADDRESS } = require('./common');

// Constants
const RELAYER_EXECUTE_COST = '0.3';
const OP_RELAYER_EXECUTE = 0x00000008;
const BYTES_PER_CELL = 96;

function bufferToCell(buffer) {
    function buildCellChain(startIndex) {
        const builder = beginCell();
        const endIndex = Math.min(startIndex + BYTES_PER_CELL, buffer.length);

        for (let i = startIndex; i < endIndex; i++) {
            builder.storeUint(buffer[i], 8);
        }

        if (endIndex < buffer.length) {
            const nextCell = buildCellChain(endIndex);
            builder.storeRef(nextCell);
        }

        return builder.endCell();
    }

    return buildCellChain(0);
}

function buildRelayerExecuteMessageBody(
    messageString,
    relayerAddress,
    sourceChainString,
    sourceAddressString,
    payloadBuffer,
    contractAddress,
    destinationChainString,
    payloadHash,
) {
    const messageIdCell = bufferToCell(Buffer.from(messageString, 'utf8'));
    const sourceChain = bufferToCell(Buffer.from(sourceChainString, 'utf8'));
    const sourceContractAddress = bufferToCell(Buffer.from(sourceAddressString, 'utf8'));
    const payload = bufferToCell(payloadBuffer);
    const destinationChain = bufferToCell(Buffer.from(destinationChainString, 'utf8'));
    const contractAddressBuffer = contractAddress.hash;
    const contractAddressCell = bufferToCell(contractAddressBuffer);

    const message = beginCell()
        .storeRef(messageIdCell)
        .storeRef(sourceChain)
        .storeRef(sourceContractAddress)
        .storeRef(
            beginCell()
                .storeRef(payload)
                .storeRef(contractAddressCell)
                .storeRef(destinationChain)
                .storeUint(payloadHash, 256)
                .endCell(),
        )
        .endCell();

    return beginCell()
        .storeUint(OP_RELAYER_EXECUTE, 32)
        .storeRef(message)
        .storeAddress(relayerAddress)
        .endCell();
}

async function run(messageId, sourceChain, sourceAddress, payload, executableAddress, destinationChain, payloadHash) {
    try {
        const client = getTonClient();
        const { contract, key, wallet } = await loadWallet(client);
        const gateway = Address.parse(GATEWAY_ADDRESS);

        const payloadBuffer = Buffer.from(payload, 'hex');
        const executableAddr = Address.parseRaw(executableAddress);

        const relayerExecuteCell = buildRelayerExecuteMessageBody(
            messageId,
            wallet.address,
            sourceChain,
            sourceAddress,
            payloadBuffer,
            executableAddr,
            destinationChain,
            BigInt(payloadHash)
        );

        const message = internal({
            to: gateway,
            value: RELAYER_EXECUTE_COST,
            body: relayerExecuteCell,
        });

        const seqno = await contract.getSeqno();
        console.log('Current wallet seqno:', seqno);

        console.log('Sending relayer execute transaction...');
        const transfer = await contract.sendTransfer({
            secretKey: key.secretKey,
            messages: [message],
            seqno: seqno,
            amount: RELAYER_EXECUTE_COST,
        });

        console.log('Relayer execute transaction sent successfully!');

        await waitForTransaction(contract, seqno);

    } catch (error) {
        console.error('Error in relayer execute:', error);
        throw error;
    }
}

// Set up command line interface
if (require.main === module) {
    const program = new Command();
    program
        .name('relayerExecute')
        .description('Execute relayer message on TON gateway')
        .argument('<messageId>', 'Message ID (e.g. 0x678771abd95ff19d3285e1a43a25a5e5f4e5c8e4dcabec0e1cb342bc18c63366-0)')
        .argument('<sourceChain>', 'Source chain name (e.g. avalanche-fuji)')
        .argument('<sourceAddress>', 'Source address (e.g. 0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC)')
        .argument('<payload>', 'Payload in hex (e.g. 48656c6c6f2066726f6d204176616c616e63686521)')
        .argument('<executableAddress>', 'Executable contract address (e.g. 0:4a1a80a7b0326b22310dced59d8b52efddf313e77f9b48f226b69b8efedbe24d)')
        .argument('<destinationChain>', 'Destination chain name (e.g. ton)')
        .argument('<payloadHash>', 'Payload hash (e.g. 0x35d25b76a49eebc07a7419b922fc11bd7bba1970b579d2a380ddd6606c5a1ff8)')
        .action(run);

    program.parse();
}
