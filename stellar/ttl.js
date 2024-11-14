'use strict';

const { Contract, SorobanRpc, SorobanDataBuilder, TransactionBuilder, Operation } = require('@stellar/stellar-sdk');
const { Server } = require('@stellar/stellar-sdk/rpc');
const { Command } = require('commander');
const { execSync } = require('child_process');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { stellarCmd, getNetworkPassphrase, getWallet, addBaseOptions } = require('./utils');
const { getChainConfig, addOptionsToCommands } = require('../common');
const { prompt } = require('../common/utils');
require('./cli-utils');

const MAX_INSTANCE_TTL_EXTENSION = 535679;

async function getTtl(chain, contractName, _args, _options) {
    printInfo('get ttl', contractName);
    const ledgerEntry = await getLedgerEntry(chain, contractName);
    printInfo('latest ledger', ledgerEntry.latestLedger);
    printInfo('live until ledger', ledgerEntry.entries[0].liveUntilLedgerSeq);
}

async function getLedgerEntry(chain, contractName) {
    const contract = new Contract(chain.contracts[contractName].address);
    const instance = contract.getFootprint();
    const server = new SorobanRpc.Server(chain.rpc);
    return await server.getLedgerEntries(...[instance]);
}

async function extendInstance(chain, contractName, _args, options) {
    const { yes } = options;
    const { rpc, networkType } = chain;

    if (prompt(`Extend instance ttl for ${contractName}`, yes)) {
        return;
    }

    const server = new Server(rpc);
    const wallet = await getWallet(chain, options);

    const account = await server.getAccount(wallet.publicKey());
    const fee = '200100'; // Base fee plus resource fee

    const contract = new Contract(chain.contracts[contractName].address);
    const instance = contract.getFootprint();

    const sorobanData = new SorobanDataBuilder().setResourceFee(200000).setReadOnly([instance]).build();
    const transaction = new TransactionBuilder(account, {
        fee,
        networkPassphrase: getNetworkPassphrase(networkType),
    })
        .setSorobanData(sorobanData)
        .addOperation(
            Operation.extendFootprintTtl({
                extendTo: MAX_INSTANCE_TTL_EXTENSION,
            }),
        )
        .setTimeout(30)
        .build();

    transaction.sign(wallet);
    await server.sendTransaction(transaction);
}

async function restoreInstance(chain, contractName, _args, options) {
    const { yes } = options;
    const { rpc, networkType } = chain;

    if (prompt(`Restore instance for ${contractName}`, yes)) {
        return;
    }

    const networkPassphrase = getNetworkPassphrase(networkType);
    const contractId = chain.contracts[contractName].address;

    const cmd = `${stellarCmd} contract restore --id ${contractId} --source-account wallet --network ${networkType} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}"`;

    execSync(cmd, { stdio: 'inherit' });
}

async function mainProcessor(processor, contractName, args, options) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);

    if (!chain.contracts[contractName]) {
        throw new Error('Contract not found');
    }

    await processor(chain, contractName, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('ttl').description('Manage contract instance and storage `time to live`');

    program
        .command('get-ttl <contractName>')
        .description('Get the current ttl of a contract instance')
        .action((contractName, options) => {
            mainProcessor(getTtl, contractName, [], options);
        });

    program
        .command('extend-instance <contractName>')
        .description('Extend the ttl for a contract instance and its wasm code')
        .action((contractName, options) => {
            mainProcessor(extendInstance, contractName, [], options);
        });

    program
        .command('restore-instance <contractName>')
        .description('Restore an archived contract instance')
        .action((contractName, options) => {
            mainProcessor(restoreInstance, contractName, [], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
