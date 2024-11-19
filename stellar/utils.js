'use strict';

const {
    Keypair,
    rpc,
    Horizon,
    TransactionBuilder,
    Networks,
    BASE_FEE,
    xdr: { DiagnosticEvent, SorobanTransactionData },
    Address,
} = require('@stellar/stellar-sdk');
const { printInfo, sleep, addEnvOption } = require('../common');
const { Option } = require('commander');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { ethers } = require('hardhat');
const {
    utils: { arrayify },
    BigNumber,
} = ethers;

const stellarCmd = 'soroban';
const ASSET_TYPE_NATIVE = 'native';

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

const addBaseOptions = (program, options = {}) => {
    addEnvOption(program);
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(new Option('--chain-name <chainName>', 'chain name for stellar in amplifier').default('stellar').env('CHAIN'));
    program.addOption(new Option('-v, --verbose', 'verbose output').default(false));
    program.addOption(new Option('--estimate-cost', 'estimate on-chain resources').default(false));

    if (!options.ignorePrivateKey) {
        program.addOption(new Option('-p, --private-key <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    }

    if (options.address) {
        program.addOption(new Option('--address <address>', 'override contract address'));
    }

    return program;
};

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

async function sendTransaction(tx, server, action, options = {}) {
    // Submit the transaction to the Soroban-RPC server. The RPC server will
    // then submit the transaction into the network for us. Then we will have to
    // wait, polling `getTransaction` until the transaction completes.
    try {
        const sendResponse = await server.sendTransaction(tx);
        printInfo(`${action} Tx`, sendResponse.hash);

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

async function broadcast(operation, wallet, chain, action, options = {}) {
    const server = new rpc.Server(chain.rpc);

    if (options.estimateCost) {
        const tx = await buildTransaction(operation, server, wallet, chain.networkType, options);
        const resourceCost = await estimateCost(tx, server);
        printInfo('Gas cost', JSON.stringify(resourceCost, null, 2));
        return;
    }

    const tx = await prepareTransaction(operation, server, wallet, chain.networkType, options);
    return await sendTransaction(tx, server, action, options);
}

function getAssetCode(balance, chain) {
    return balance.asset_type === 'native' ? chain.tokenSymbol : balance.asset_code;
}

async function getWallet(chain, options) {
    const keypair = Keypair.fromSecret(options.privateKey);
    const address = keypair.publicKey();
    const provider = new rpc.Server(chain.rpc);
    const horizonServer = new Horizon.Server(chain.horizonRpc);
    const balances = await getBalances(horizonServer, address);

    printInfo('Wallet address', address);
    printInfo('Wallet balances', balances.map((balance) => `${balance.balance} ${getAssetCode(balance, chain)}`).join('  '));
    printInfo('Wallet sequence', await provider.getAccount(address).then((account) => account.sequenceNumber()));

    return keypair;
}

async function getBalances(horizonServer, address) {
    const response = await horizonServer
        .accounts()
        .accountId(address)
        .call()
        .catch((error) => {
            if (error?.response?.status === 404) {
                return { balances: [] };
            }

            throw error;
        });
    return response.balances;
}

async function estimateCost(tx, server) {
    await server.simulateTransaction(tx);

    const response = await server._simulateTransaction(tx);

    if (response.error) {
        throw new Error(response.error);
    }

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

const getAmplifierVerifiers = async (config, chainAxelarId) => {
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    const { id: verifierSetId, verifier_set: verifierSet } = await client.queryContractSmart(
        config.axelar.contracts.MultisigProver[chainAxelarId].address,
        'current_verifier_set',
    );
    const signers = Object.values(verifierSet.signers);

    const weightedSigners = signers
        .map((signer) => ({
            signer: Address.account(Buffer.from(arrayify(`0x${signer.pub_key.ed25519}`))).toString(),
            weight: Number(signer.weight),
        }))
        .sort((a, b) => a.signer.localeCompare(b.signer));

    return {
        signers: weightedSigners,
        threshold: Number(verifierSet.threshold),
        nonce: arrayify(ethers.utils.hexZeroPad(BigNumber.from(verifierSet.created_at).toHexString(), 32)),
        verifierSetId,
    };
};

function serializeValue(value) {
    if (value instanceof Uint8Array) {
        return Buffer.from(value).toString('hex');
    }

    if (Array.isArray(value)) {
        return value.map(serializeValue);
    }

    if (typeof value === 'bigint') {
        return value.toString();
    }

    if (typeof value === 'object') {
        return Object.entries(value).reduce((acc, [key, val]) => {
            acc[key] = serializeValue(val);
            return acc;
        }, {});
    }

    return value;
}

module.exports = {
    stellarCmd,
    ASSET_TYPE_NATIVE,
    buildTransaction,
    prepareTransaction,
    sendTransaction,
    broadcast,
    getWallet,
    estimateCost,
    getNetworkPassphrase,
    addBaseOptions,
    getAmplifierVerifiers,
    serializeValue,
    getBalances,
};
