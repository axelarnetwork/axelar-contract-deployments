const SafeModule = require('@safe-global/protocol-kit');
const Safe = SafeModule?.default || SafeModule.Safe;

async function executeSafeTransaction({ rpc, multisigAddress, tokenAddress, functionCall, gasPayment, privateKey1, privateKey2 }) {
    // Owner 1 creates and signs Safe tx
    const safe1 = await Safe.init({ provider: rpc, signer: privateKey1, safeAddress: multisigAddress });
    const safeTx = await safe1.createTransaction({
        transactions: [{ to: tokenAddress, data: functionCall, value: gasPayment.toString() }],
        options: {
            safeTxGas: 300000, // give Safe execution room (can adjust higher if needed)
        },
    });
    const safeTxSignedBy1 = await safe1.signTransaction(safeTx);

    // Owner 2 adds signature and executes
    const safe2 = await Safe.init({ provider: rpc, signer: privateKey2, safeAddress: multisigAddress });
    const safeTxSignedBy2 = await safe2.signTransaction(safeTxSignedBy1);
    const exec = await safe2.executeTransaction(safeTxSignedBy2);
    const execReceipt = await exec.transactionResponse.wait();

    return {
        hash: exec.hash,
        receipt: execReceipt,
        blockNumber: execReceipt.blockNumber,
    };
}

module.exports = {
    executeSafeTransaction,
};
