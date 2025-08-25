'use strict';

const { Contract, SorobanRpc } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { execSync } = require('child_process');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { stellarCmd, getNetworkPassphrase, addBaseOptions, getWallet, broadcast, serializeValue, addressToScVal } = require('./utils');
const { getChainConfig, addOptionsToCommands } = require('../common');
const { prompt, validateParameters } = require('../common/utils');
require('./cli-utils');

const MAX_INSTANCE_TTL_EXTENSION = 535679;

async function submitOperation(wallet, chain, _contractName, contract, args, options, operation = '') {
    if (!operation) {
        operation = args.operation;
    }

    const callOperation = Array.isArray(args) ? await contract.call(operation, ...args) : await contract.call(operation);

    const response = await broadcast(callOperation, wallet, chain, `${operation}`, options);
    const result = response.value();

    if (result !== undefined) {
        printInfo(`${_contractName}:${operation} returned`, serializeValue(result));
    } else {
        printInfo(`${_contractName}:${operation} succeeded`);
    }
}

async function transferOwnership(wallet, chain, _contractName, contract, args, options) {
    return submitOperation(wallet, chain, _contractName, contract, [addressToScVal(args)], options, 'transfer_ownership');
}

async function transferOperatorship(wallet, chain, _contractName, contract, args, options) {
    return submitOperation(wallet, chain, _contractName, contract, [addressToScVal(args)], options, 'transfer_operatorship');
}

async function getTtl(_wallet, chain, contractName, contract, _args, _options) {
    printInfo('Contract TTL', contractName);
    const ledgerEntry = await getLedgerEntry(chain, contract);
    printInfo('Latest Ledger', ledgerEntry.latestLedger);
    printInfo('Expiry Ledger', ledgerEntry.entries[0].liveUntilLedgerSeq);
}

async function getLedgerEntry(chain, contract) {
    const instance = contract.getFootprint();
    const server = new SorobanRpc.Server(chain.rpc);
    return server.getLedgerEntries(...[instance]);
}

async function extendInstance(_wallet, chain, contractName, _contract, _args, options) {
    const { rpc, networkType } = chain;
    const ledgersToExtend = !options.extendBy ? MAX_INSTANCE_TTL_EXTENSION : options.extendBy;

    const networkPassphrase = getNetworkPassphrase(networkType);
    const contractId = chain.contracts[contractName].address;

    validateParameters({
        isValidStellarAddress: { contractId },
    });

    const cmd = `${stellarCmd} contract extend --id ${contractId} --source-account wallet --network ${networkType} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}" --ledgers-to-extend ${ledgersToExtend}`;

    execSync(cmd, { stdio: 'inherit' });
}

async function restoreInstance(_wallet, chain, contractName, _contract, _args, _options) {
    const { rpc, networkType } = chain;
    const networkPassphrase = getNetworkPassphrase(networkType);
    const contractId = chain.contracts[contractName].address;

    validateParameters({
        isValidStellarAddress: { contractId },
    });

    const cmd = `${stellarCmd} contract restore --id ${contractId} --source-account wallet --network ${networkType} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}"`;

    execSync(cmd, { stdio: 'inherit' });
}

async function mainProcessor(processor, contractName, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

    if (!chain.contracts[contractName]) {
        throw new Error('Contract not found');
    }

    const contractId = chain.contracts[contractName].address;
    const contract = new Contract(chain.contracts[contractName].address);

    validateParameters({
        isValidStellarAddress: { contractId },
    });
    await processor(wallet, chain, contractName, contract, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('contract').description('Common contract operations');

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

    program
        .command('paused')
        .description('Check if the contract is paused')
        .argument('<contract-name>', 'contract name to check paused')
        .action((contractName, options) => {
            mainProcessor(submitOperation, contractName, { operation: 'paused' }, options);
        });

    program
        .command('pause')
        .description('Pause the contract')
        .argument('<contract-name>', 'contract name to pause')
        .action((contractName, options) => {
            mainProcessor(submitOperation, contractName, { operation: 'pause' }, options);
        });

    program
        .command('unpause')
        .description('Unpause the contract')
        .argument('<contract-name>', 'contract name to unpause')
        .action((contractName, options) => {
            mainProcessor(submitOperation, contractName, { operation: 'unpause' }, options);
        });

    program
        .command('owner')
        .description('Retrieve the owner of the contract')
        .argument('<contract-name>', 'contract name')
        .action((contractName, options) => {
            mainProcessor(submitOperation, contractName, { operation: 'owner' }, options);
        });

    program
        .command('transfer-ownership <contractName> <newOwner>')
        .description('Transfer the ownership of the contract')
        .action((contractName, newOwner, options) => {
            mainProcessor(transferOwnership, contractName, newOwner, options);
        });

    program
        .command('operator')
        .description('Retrieve the operator of the contract')
        .argument('<contract-name>', 'contract name')
        .action((contractName, options) => {
            mainProcessor(submitOperation, contractName, { operation: 'operator' }, options);
        });

    program
        .command('transfer-operatorship <contractName> <newOperator>')
        .description('Transfer the operatorship of the contract')
        .action((contractName, newOperator, options) => {
            mainProcessor(transferOperatorship, contractName, newOperator, options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
