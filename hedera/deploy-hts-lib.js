'use strict';

const { Command, Option } = require('commander');
const { ContractCreateFlow } = require('@hashgraph/sdk');
const { getContractJSON } = require('../evm/utils.js');
const { getClient } = require('./client.js');
const { addBaseOptions } = require('./cli-utils');
const { HTS_LIBRARY_NAME } = require('./utils.js');
const { printInfo } = require('../common/utils');

const DEFAULT_GAS_LIMIT = 3_000_000;

function contractIdToEvmAddress(shard, realm, num) {
    const buf = Buffer.alloc(20);
    buf.writeUInt32BE(shard, 0);
    buf.writeBigUInt64BE(BigInt(realm), 4);
    buf.writeBigUInt64BE(BigInt(num), 12);
    return '0x' + buf.toString('hex');
}

async function deployHtsLib(_config, options) {
    const client = await getClient(options.accountId, options.privateKey, options.hederaNetwork);

    const contractName = HTS_LIBRARY_NAME;
    const gasLimit = options.gas || DEFAULT_GAS_LIMIT;

    const json = getContractJSON(contractName);
    const bytecode = json.bytecode;

    printInfo(`Deploying ${contractName} library`, json.sourceName);
    printInfo(`Using gas limit`, gasLimit);

    // Create the transaction
    const contractCreate = new ContractCreateFlow().setGas(gasLimit).setBytecode(bytecode);

    // Sign the transaction with the client operator key and submit to a Hedera network
    const txResponse = await contractCreate.execute(client);
    printInfo(`Txid`, txResponse.transactionId);

    // Get the receipt of the transaction
    const receipt = await txResponse.getReceipt(client);

    // Get the new contract ID
    const newContractId = receipt.contractId;
    printInfo('The new contract ID', newContractId);

    const evmAddress = contractIdToEvmAddress(newContractId.shard, newContractId.realm, newContractId.num);

    printInfo(`EVM address of the new contract`, evmAddress);

    if (options.output) {
        const fs = require('fs');
        const output = {
            contractId: newContractId.toString(),
            evmAddress,
            contractName,
            sourceName: json.sourceName,
            deployedAt: new Date().toISOString(),
        };
        fs.writeFileSync(options.output, JSON.stringify(output, null, 2));
        printInfo(`Deployment info saved to`, options.output);
    }
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-hts-lib')
        .description('Deploy HTS library contract to Hedera')
        .addOption(
            new Option('--gas <gas>', 'gas limit for deployment').default(DEFAULT_GAS_LIMIT).argParser((value) => parseInt(value, 10)),
        )
        .addOption(new Option('--output <output>', 'output file path to save deployment info'))
        .action((options) => {
            deployHtsLib(null, options);
        });

    addBaseOptions(program);

    program.parse();
}

module.exports = {
    deployHtsLib,
    contractIdToEvmAddress,
};
