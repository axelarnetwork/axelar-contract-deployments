'use strict';

const {
    Keypair,
    SorobanRpc,
    Horizon,
    TransactionBuilder,
    Networks,
    BASE_FEE,
    xdr: { DiagnosticEvent, SorobanTransactionData },
} = require('@stellar/stellar-sdk');
const { printInfo, sleep } = require('../evm/utils');

function getNetworkPassphrase(networkType) {
    switch (networkType) {
        case 'local':
            return Networks.SANDBOX;
        case 'futurenet':
            return Networks.FUTURENET;
        case 'testnet':
            return Networks.TESTNET;
        case 'mainnet':
            return Networks.PUBLIC;
        default:
            throw new Error(`Unknown network type: ${networkType}`);
    }
}

async function buildTransaction(operation, server, wallet, networkType, options = {}) {
    const account = await server.getAccount(wallet.publicKey());
    const networkPassphrase = getNetworkPassphrase(networkType);
    const builtTransaction = new TransactionBuilder(account, {
        fee: BASE_FEE,
        networkPassphrase,
    })
        .addOperation(operation)
        .setTimeout(options.timeout || 30)
        .build();

    if (options.verbose) {
        printInfo('Tx', builtTransaction.toXDR());
    }

    return builtTransaction;
}

const prepareTransaction = async (operation, server, wallet, networkType, options = {}) => {
    const builtTransaction = await buildTransaction(operation, server, wallet, networkType, options);

    // We use the RPC server to "prepare" the transaction. This simulating the
    // transaction, discovering the storage footprint, and updating the
    // transaction to include that footprint. If you know the footprint ahead of
    // time, you could manually use `addFootprint` and skip this step.
    const preparedTransaction = await server.prepareTransaction(builtTransaction);

    preparedTransaction.sign(wallet);

    if (options.verbose) {
        printInfo('Signed tx', preparedTransaction.toEnvelope().toXDR('base64'));
    }

    return preparedTransaction;
};

async function sendTransaction(tx, server, options = {}) {
    // Submit the transaction to the Soroban-RPC server. The RPC server will
    // then submit the transaction into the network for us. Then we will have to
    // wait, polling `getTransaction` until the transaction completes.
    try {
        const sendResponse = await server.sendTransaction(tx);
        printInfo('Transaction hash', '0x' + sendResponse.hash);

        if (options.verbose) {
            printInfo('Transaction broadcast response', JSON.stringify(sendResponse));
        }

        if (sendResponse.status !== 'PENDING') {
            throw Error(sendResponse.errorResultXdr);
        }

        let getResponse = await server.getTransaction(sendResponse.hash);
        const retryWait = 1000; // 1 sec
        let retries = 10;

        while (getResponse.status === 'NOT_FOUND' && retries > 0) {
            await sleep(retryWait);

            getResponse = await server.getTransaction(sendResponse.hash);

            retries -= 1;
        }

        if (options.verbose) {
            printInfo('Transaction response', JSON.stringify(getResponse));
        }

        if (getResponse.status !== 'SUCCESS') {
            throw Error(`Transaction failed: ${getResponse.resultXdr}`);
        }

        // Make sure the transaction's resultMetaXDR is not empty
        // TODO: might be empty if the operation doesn't have a return value
        if (!getResponse.resultMetaXdr) {
            throw Error('Empty resultMetaXDR in getTransaction response');
        }

        const transactionMeta = getResponse.resultMetaXdr;
        const returnValue = transactionMeta.v3().sorobanMeta().returnValue();

        if (options.verbose) {
            printInfo('Transaction result', returnValue.value());
        }

        return returnValue.value();
    } catch (err) {
        console.log('Sending transaction failed');
        throw err;
    }
}

function getAssetCode(balance, chain) {
    return balance.asset_type === 'native' ? chain.tokenSymbol : balance.asset_code;
}

async function getWallet(chain, options) {
    const keypair = Keypair.fromSecret(options.privateKey);
    const address = keypair.publicKey();
    const provider = new SorobanRpc.Server(chain.rpc);
    const horizonServer = new Horizon.Server(chain.horizonRpc);

    printInfo('Wallet address', address);
    const account = await provider.getAccount(address);

    const { balances } = await horizonServer.accounts().accountId(address).call();
    printInfo('Wallet Balances', balances.map((balance) => `${balance.balance} ${getAssetCode(balance, chain)}`).join('  '));

    printInfo('Wallet sequence', account.sequenceNumber());

    return [keypair, provider];
}

async function estimateCost(tx, server) {
    await server.simulateTransaction(tx);

    const response = await server._simulateTransaction(tx);

    const events = response.events.map((event) => {
        const e = DiagnosticEvent.fromXDR(event, 'base64');

        if (e.event().type().name === 'diagnostic') return 0;

        return e.toXDR().length;
    });

    const eventsAndReturnValueSize =
        events.reduce((accumulator, currentValue) => accumulator + currentValue, 0) + // events
        Buffer.from(response.results[0].xdr, 'base64').length; // return value size

    const sorobanTransactionData = SorobanTransactionData.fromXDR(response.transactionData, 'base64');

    return {
        // the first two lines are incorrect. use sorobanTransactionData instead of `cost`
        cpu_instructions: Number(response.cost.cpuInsns),
        ram: Number(response.cost.memBytes),

        min_resource_fee: response.minResourceFee,
        ledger_read_bytes: sorobanTransactionData.resources().readBytes(),
        ledger_write_bytes: sorobanTransactionData.resources().writeBytes(),
        ledger_entry_reads: sorobanTransactionData.resources().footprint().readOnly().length,
        ledger_entry_writes: sorobanTransactionData.resources().footprint().readWrite().length,
        events_and_return_value_size: eventsAndReturnValueSize,
        transaction_size: Buffer.from(response.transactionData, 'base64').length,
    };
}

module.exports = {
    buildTransaction,
    prepareTransaction,
    sendTransaction,
    getWallet,
    estimateCost,
    getNetworkPassphrase,
};
