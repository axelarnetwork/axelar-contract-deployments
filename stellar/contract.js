'use strict';

const { Contract, SorobanRpc } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { execSync } = require('child_process');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { stellarCmd, getNetworkPassphrase, addBaseOptions } = require('./utils');
const { getChainConfig, addOptionsToCommands } = require('../common');
const { prompt } = require('../common/utils');
require('./cli-utils');

const MAX_INSTANCE_TTL_EXTENSION = 535679;

async function getTtl(chain, contractName, _args, _options) {
    printInfo('Contract TTL', contractName);
    const ledgerEntry = await getLedgerEntry(chain, contractName);
    printInfo('Latest Ledger', ledgerEntry.latestLedger);
    printInfo('Expiry Ledger', ledgerEntry.entries[0].liveUntilLedgerSeq);
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

    const ledgersToExtend = !options.extendBy ? MAX_INSTANCE_TTL_EXTENSION : options.extendBy;

    const networkPassphrase = getNetworkPassphrase(networkType);
    const contractId = chain.contracts[contractName].address;

    const cmd = `${stellarCmd} contract extend --id ${contractId} --source-account wallet --network ${networkType} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}" --ledgers-to-extend ${ledgersToExtend}`;

    execSync(cmd, { stdio: 'inherit' });
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

    program.name('contract').description('Manage contract instance and storage `time to live`');

    program
        .command('get-ttl <contractName>')
        .description('Get the current ttl of a contract instance')
        .action((contractName, options) => {
            mainProcessor(getTtl, contractName, [], options);
        });

    program
        .command('extend-instance <contractName>')
        .description('Extend the ttl for a contract instance and its wasm code')
        .addOption(
            new Option(
                '--extend-by <extendBy>',
                'Number of ledgers to extend by. If ommitted, will default to the maximum extension amount',
            ),
        )
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
