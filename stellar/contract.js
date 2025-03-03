'use strict';

const { Address, Contract, SorobanRpc } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { execSync } = require('child_process');
const { loadConfig, printInfo, saveConfig } = require('../evm/utils');
const { stellarCmd, getNetworkPassphrase, addBaseOptions, getWallet, broadcast, serializeValue, addressToScVal } = require('./utils');
const { getChainConfig, addOptionsToCommands } = require('../common');
const { prompt } = require('../common/utils');
require('./cli-utils');

const MAX_INSTANCE_TTL_EXTENSION = 535679;

async function submitOperation(chain, _, contract, operation, options) {
    const wallet = await getWallet(chain, options);
    const callOperation = await contract.call(operation);

    return broadcast(callOperation, wallet, chain, `${operation}`, options);
}

async function commonOperation(chain, _, contract, operation, options) {
    const returnValue = await submitOperation(chain, _, contract, operation, options);

    printInfo('Return value', serializeValue(returnValue.value()));
}

async function owner(chain, _, contract, operation, options) {
    const returnValue = await submitOperation(chain, _, contract, operation, options);
    const ownerScAddress = returnValue.value();

    printInfo('Owner', Address.fromScAddress(ownerScAddress).toString());
}

async function transferOwnership(chain, _, contract, args, options) {
    const newOwner = args;
    const wallet = await getWallet(chain, options);
    const operation = contract.call('transfer_ownership', addressToScVal(newOwner));

    await broadcast(operation, wallet, chain, 'transfer_ownership', options);
}

async function operator(chain, _, contract, operation, options) {
    const returnValue = await submitOperation(chain, _, contract, operation, options);
    const operatorScAddress = returnValue.value();

    printInfo('Operator', Address.fromScAddress(operatorScAddress).toString());
}

async function transferOperatorship(chain, _, contract, args, options) {
    const newOperator = args;
    const wallet = await getWallet(chain, options);
    const operation = contract.call('transfer_operatorship', addressToScVal(newOperator));

    await broadcast(operation, wallet, chain, 'transfer_operatorship', options);
}

async function getTtl(chain, contractName, contract, _args, _options) {
    printInfo('Contract TTL', contractName);
    const ledgerEntry = await getLedgerEntry(chain, contract);
    printInfo('Latest Ledger', ledgerEntry.latestLedger);
    printInfo('Expiry Ledger', ledgerEntry.entries[0].liveUntilLedgerSeq);
}

async function getLedgerEntry(chain, contract) {
    const instance = contract.getFootprint();
    const server = new SorobanRpc.Server(chain.rpc);
    return await server.getLedgerEntries(...[instance]);
}

async function extendInstance(chain, contractName, _, _args, options) {
    const { rpc, networkType } = chain;
    const ledgersToExtend = !options.extendBy ? MAX_INSTANCE_TTL_EXTENSION : options.extendBy;

    const networkPassphrase = getNetworkPassphrase(networkType);
    const contractId = chain.contracts[contractName].address;

    const cmd = `${stellarCmd} contract extend --id ${contractId} --source-account wallet --network ${networkType} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}" --ledgers-to-extend ${ledgersToExtend}`;

    execSync(cmd, { stdio: 'inherit' });
}

async function restoreInstance(chain, contractName, _, _args, options) {
    const { rpc, networkType } = chain;
    const networkPassphrase = getNetworkPassphrase(networkType);
    const contractId = chain.contracts[contractName].address;

    const cmd = `${stellarCmd} contract restore --id ${contractId} --source-account wallet --network ${networkType} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}"`;

    execSync(cmd, { stdio: 'inherit' });
}

async function mainProcessor(processor, contractName, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

    if (!chain.contracts[contractName]) {
        throw new Error('Contract not found');
    }

    const contract = new Contract(chain.contracts[contractName].address);

    await processor(chain, contractName, contract, args, options);

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
            mainProcessor(commonOperation, contractName, 'paused', options);
        });

    program
        .command('pause')
        .description('Pause the contract')
        .argument('<contract-name>', 'contract name to pause')
        .action((contractName, options) => {
            mainProcessor(commonOperation, contractName, 'pause', options);
        });

    program
        .command('unpause')
        .description('Unpause the contract')
        .argument('<contract-name>', 'contract name to unpause')
        .action((contractName, options) => {
            mainProcessor(commonOperation, contractName, 'unpause', options);
        });

    program
        .command('owner')
        .description('Retrieve the owner of the contract')
        .argument('<contract-naame>', 'contract name')
        .action((contractName, options) => {
            mainProcessor(owner, contractName, 'owner', options);
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
        .argument('<contract-naame>', 'contract name')
        .action((contractName, options) => {
            mainProcessor(operator, contractName, 'operator', options);
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
