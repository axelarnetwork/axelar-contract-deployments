#!/usr/bin/env node
const { Command } = require('commander');
const { toNano, Address, beginCell } = require('@ton/ton');
const { getTonClient, loadWallet, waitForTransaction, sendTransactionWithCost } = require('./common');
const {
    buildPayNativeGasForContractCallMessage,
    buildPayJettonGasForContractCallMessage,
    buildCollectGasMessage,
    buildCollectJettonsMessage,
    buildAddNativeGasMessage,
    buildAddJettonGasMessage,
    buildNativeRefundMessage,
    buildJettonRefundMessage,
    buildUpdateGasInfoMessage,
    buildPayGasMessage,
    JettonWallet,
    JettonMinter,
} = require('@commonprefix/axelar-cgp-ton');
const { keccak256 } = require('@ethersproject/keccak256');

const GAS_SERVICE_ADDRESS = process.env.TON_GAS_SERVICE_ADDRESS;

if (!GAS_SERVICE_ADDRESS) {
    throw new Error('Please set TON_GAS_SERVICE_ADDRESS in your .env file');
}

const program = new Command();
program.name('gasService').description('Axelar TON Gas Service CLI').version('1.0.0');

async function executeOperation(operationName, messageBody, cost) {
    try {
        const client = getTonClient();
        const { contract, key } = await loadWallet(client);

        const { transfer, seqno } = await sendTransactionWithCost(contract, key, GAS_SERVICE_ADDRESS, messageBody, cost);

        console.log(`✅ ${operationName} transaction sent successfully!`);
        await waitForTransaction(contract, seqno);
    } catch (error) {
        console.error(`❌ Error in ${operationName}:`, error.message);
        process.exit(1);
    }
}

// Helper function to send jettons with bundled operations
async function sendJettonsTo(receiver, deployerJettonWallet, jettonToSend, forwardPayload) {
    const client = getTonClient();
    const { contract, key } = await loadWallet(client);

    return await deployerJettonWallet.sendTransfer(
        client.provider(deployerJettonWallet.address),
        contract.sender(key.secretKey),
        toNano('0.08'), // transaction fee
        jettonToSend, // amount of jettons to send
        receiver, // the destination address
        receiver, // responseAddress (can be your deployer address)
        beginCell().endCell(), // custom payload
        toNano('0.04'), // forward_ton_amount
        forwardPayload, // forwardPayload
    );
}

// 1. Pay Native Gas For Contract Call
program
    .command('pay-native-gas')
    .description('Pay native TON gas for contract call')
    .argument('<destination-chain>', 'Destination chain name')
    .argument('<destination-address>', 'Destination contract address')
    .argument('<payload>', 'Payload in hex format')
    .argument('<refund-address>', 'Refund address')
    .argument('<gas-amount>', 'Gas amount in TON')
    .action(async (destinationChain, destinationAddress, payload, refundAddress, gasAmount) => {
        const cost = toNano(gasAmount);

        const client = getTonClient();
        const { contract, key } = await loadWallet(client);
        const sender = contract.address;

        await executeOperation(
            'Pay Native Gas',
            buildPayNativeGasForContractCallMessage(sender, destinationChain, destinationAddress, payload, Address.parse(refundAddress)),
            cost,
        );
    });

// 2. Pay Jetton Gas For Contract Call (Bundled)
program
    .command('pay-jetton-gas')
    .description('Pay jetton gas for contract call')
    .argument('<jetton-minter>', 'Jetton minter address')
    .argument('<jetton-amount>', 'Jetton amount')
    .argument('<destination-chain>', 'Destination chain name')
    .argument('<destination-address>', 'Destination contract address')
    .argument('<payload>', 'Payload in hex format')
    .action(async (jettonMinter, jettonAmount, destinationChain, destinationAddress, payload) => {
        try {
            const client = getTonClient();
            const { contract, key } = await loadWallet(client);
            const sender = contract.address;
            const refundAddress = sender;

            const gasServiceAddress = Address.parse(GAS_SERVICE_ADDRESS);
            const jettonMinterAddress = Address.parse(jettonMinter);

            const minter = JettonMinter.createFromAddress(jettonMinterAddress);
            const jettonWalletAddress = await minter.getWalletAddress(client.provider(jettonMinterAddress), sender);

            const jettonToSend = jettonAmount;
            const userJettonWallet = JettonWallet.createFromAddress(jettonWalletAddress);

            // Create the pay jetton gas message
            const payJettonGasMessage = buildPayJettonGasForContractCallMessage(
                destinationChain,
                destinationAddress,
                payload,
                sender,
                refundAddress,
            );

            // Create the bundled forward payload
            const forwardPayload = beginCell().storeAddress(jettonMinterAddress).storeRef(payJettonGasMessage).endCell();

            console.log(`Sending ${jettonToSend.toString()} Jettons with bundled gas payment`);

            const res = await sendJettonsTo(gasServiceAddress, userJettonWallet, jettonToSend, forwardPayload);

            console.log('Transaction result:', res);
            console.log('✅ Bundled jetton gas payment transaction sent successfully!');

            // Wait for confirmation
            const seqno = await contract.getSeqno();
            await waitForTransaction(contract, seqno);
        } catch (error) {
            console.error('❌ Error in bundled jetton gas payment:', error);
            console.error('Error details:', error.message);
            process.exit(1);
        }
    });

// 3. Collect Gas (Native)
program
    .command('collect-gas')
    .description('Collect native gas fees (gas collector only)')
    .argument('<receiver>', 'Receiver address')
    .argument('<amount>', 'Amount in TON to collect')
    .action(async (receiver, amount) => {
        const cost = toNano('0.01');
        const collectAmount = toNano(amount);

        const client = getTonClient();
        const { contract, key } = await loadWallet(client);
        const sender = contract.address;
        console.log('sender: ', sender);

        await executeOperation('Collect Gas', buildCollectGasMessage(Address.parse(receiver), collectAmount), cost);
    });

// 4. Collect Jettons
program
    .command('collect-jettons')
    .description('Collect jetton gas fees (gas collector only)')
    .argument('<receiver>', 'Receiver address')
    .argument('<jetton-amount>', 'Jetton amount to collect')
    .argument('<jetton-minter>', 'Jetton minter address')
    .action(async (receiver, jettonAmount, jettonMinter) => {
        const cost = toNano('0.05');
        const jettonToCollect = toNano(jettonAmount);

        await executeOperation(
            'Collect Jettons',
            buildCollectJettonsMessage(Address.parse(receiver), jettonToCollect, Address.parse(jettonMinter)),
            cost,
        );
    });

// 5. Add Native Gas
program
    .command('add-native-gas')
    .description('Add native gas to existing transaction')
    .argument('<tx-hash>', 'Transaction hash (hex without 0x prefix)')
    .argument('<refund-address>', 'Refund address')
    .argument('<gas-amount>', 'Gas amount in TON to add')
    .action(async (txHash, refundAddress, gasAmount) => {
        const cost = toNano(gasAmount);
        const txHashBigInt = BigInt('0x' + txHash);

        await executeOperation('Add Native Gas', buildAddNativeGasMessage(txHashBigInt, Address.parse(refundAddress)), cost);
    });

// 6. Add Jetton Gas (Bundled)
program
    .command('add-jetton-gas')
    .description('Add jetton gas to existing transaction')
    .argument('<tx-hash>', 'Transaction hash (hex without 0x prefix)')
    .argument('<jetton-minter>', 'Jetton minter address')
    .argument('<jetton-amount>', 'Jetton amount')
    .argument('<refund-address>', 'Refund address')
    .action(async (txHash, jettonMinter, jettonAmount, refundAddress) => {
        try {
            const txHashBigInt = BigInt('0x' + txHash);
            const jettonToAdd = jettonAmount;

            const client = getTonClient();
            const { contract, key } = await loadWallet(client);

            const gasServiceAddress = Address.parse(GAS_SERVICE_ADDRESS);
            const jettonMinterAddress = Address.parse(jettonMinter);
            const sender = contract.address;

            const minter = JettonMinter.createFromAddress(jettonMinterAddress);
            const jettonWalletAddress = await minter.getWalletAddress(client.provider(jettonMinterAddress), sender);

            const userJettonWallet = JettonWallet.createFromAddress(jettonWalletAddress);

            // Create the add jetton gas message
            const addJettonGasMessage = buildAddJettonGasMessage(txHashBigInt, Address.parse(refundAddress));

            // Create the bundled forward payload
            const forwardPayload = beginCell().storeAddress(jettonMinterAddress).storeRef(addJettonGasMessage).endCell();

            console.log(`Adding ${jettonToAdd.toString()} Jettons as gas with bundled operation`);

            const res = await sendJettonsTo(gasServiceAddress, userJettonWallet, jettonToAdd, forwardPayload);

            console.log('Transaction result:', res);
            console.log('✅ Bundled add jetton gas transaction sent successfully!');

            // Wait for confirmation
            const seqno = await contract.getSeqno();
            await waitForTransaction(contract, seqno);
        } catch (error) {
            console.error('❌ Error in bundled add jetton gas:', error);
            console.error('Error details:', error.message);
            process.exit(1);
        }
    });

// 7. Update Gas Info
program
    .command('update-gas-info')
    .description('Update gas pricing information (gas collector only)')
    .argument('<gas-info-json>', 'Gas info dictionary as JSON string')
    .argument('<gas-amount>', 'Gas amount in TON for transaction fees')
    .action(async (gasInfoJson, gasAmount) => {
        const cost = toNano(gasAmount);

        try {
            const gasInfoInput = JSON.parse(gasInfoJson);

            // Convert the input to Map<bigint, GasInfo> format
            const gasInfoMap = new Map();

            for (const [chainName, gasInfo] of Object.entries(gasInfoInput)) {
                // Hash the chain name to get the key
                const chainKey = BigInt(keccak256(Buffer.from(chainName)).toString());

                // Convert all values to bigints
                const convertedGasInfo = {
                    gasEstimationType: BigInt(gasInfo.gasEstimationType),
                    l1FeeScalar: BigInt(gasInfo.l1FeeScalar),
                    axelarBaseFee: BigInt(gasInfo.axelarBaseFee),
                    relativeGasPrice: BigInt(gasInfo.relativeGasPrice),
                    relativeBlobBaseFee: BigInt(gasInfo.relativeBlobBaseFee),
                    expressFee: BigInt(gasInfo.expressFee),
                };

                gasInfoMap.set(chainKey, convertedGasInfo);
            }

            await executeOperation('Update Gas Info', buildUpdateGasInfoMessage(gasInfoMap), cost);
        } catch (error) {
            console.error('❌ Invalid gas info JSON:', error.message);
            process.exit(1);
        }
    });

// 8. Pay Gas (with optional estimation)
program
    .command('pay-gas')
    .description('Pay gas with optional on-chain estimation')
    .argument('<destination-chain>', 'Destination chain name')
    .argument('<destination-address>', 'Destination contract address')
    .argument('<payload>', 'Payload in hex format')
    .argument('<refund-address>', 'Refund address')
    .argument('<execution-gas-limit>', 'Execution gas limit')
    .argument('<estimate-on-chain>', 'Enable on-chain gas estimation (true/false)')
    .argument('<gas-amount>', 'Gas amount in TON')
    .action(async (destinationChain, destinationAddress, payload, refundAddress, executionGasLimit, estimateOnChain, gasAmount) => {
        const cost = toNano(gasAmount);
        const estimate = Number(estimateOnChain.toLowerCase() === 'true');

        const client = getTonClient();
        const { contract, key } = await loadWallet(client);
        const sender = contract.address;

        await executeOperation(
            'Pay Gas',
            buildPayGasMessage(
                sender,
                destinationChain,
                destinationAddress,
                payload,
                Address.parse(refundAddress),
                estimate,
                toNano(executionGasLimit),
            ),
            cost,
        );
    });

// 9. Native Refund
program
    .command('native-refund')
    .description('Refund native gas (gas collector only)')
    .argument('<tx-hash>', 'Transaction hash (hex without 0x prefix)')
    .argument('<refund-address>', 'Refund address')
    .argument('<refund-amount>', 'Amount in TON to refund')
    .action(async (txHash, refundAddress, refundAmount) => {
        const cost = toNano('0.01');
        const txHashBigInt = BigInt('0x' + txHash);
        const refundAmountTon = toNano(refundAmount);

        await executeOperation(
            'Native Refund',
            buildNativeRefundMessage(txHashBigInt, Address.parse(refundAddress), refundAmountTon),
            cost,
        );
    });

// 10. Jetton Refund
program
    .command('jetton-refund')
    .description('Refund jetton gas (gas collector only)')
    .argument('<tx-hash>', 'Transaction hash (hex without 0x prefix)')
    .argument('<receiver>', 'Receiver address')
    .argument('<jetton-amount>', 'Jetton amount to refund')
    .argument('<jetton-minter>', 'Jetton minter address')
    .action(async (txHash, receiver, jettonAmount, jettonMinter) => {
        const cost = toNano('0.06');
        const txHashBigInt = BigInt('0x' + txHash);
        const jettonToSend = toNano(jettonAmount);

        await executeOperation(
            'Jetton Refund',
            buildJettonRefundMessage(txHashBigInt, Address.parse(receiver), jettonToSend, Address.parse(jettonMinter)),
            cost,
        );
    });

program.parse();
