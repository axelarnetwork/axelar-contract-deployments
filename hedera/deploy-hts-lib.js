'use strict';

const { Command, Option } = require('commander');
const { ContractCreateFlow } = require('@hashgraph/sdk');
const { getContractJSON } = require('../evm/utils.js');
const { getClient } = require('./client.js');
const { addBaseOptions } = require('./cli-utils');

function contractIdToEvmAddress(shard, realm, num) {
    const buf = Buffer.alloc(20);
    buf.writeUInt32BE(shard, 0);
    buf.writeBigUInt64BE(BigInt(realm), 4);
    buf.writeBigUInt64BE(BigInt(num), 12);
    return '0x' + buf.toString('hex');
}

async function deployHtsLib(_config, options) {
    const client = await getClient(
	    options.accountId,
	    options.privateKey,
			options.hederaNetwork,
    );

    const contractName = HTS_LIBRARY_NAME;
    const gasLimit = options.gas || 300_000;

    const json = getContractJSON(contractName);
    const bytecode = json.bytecode;

    console.log(`Deploying ${contractName} library (${json.sourceName})`);
    console.log(`Using gas limit: ${gasLimit}`);

    // Create the transaction
    const contractCreate = new ContractCreateFlow()
        .setGas(gasLimit)
        .setBytecode(bytecode);

    try {
        // Sign the transaction with the client operator key and submit to a Hedera network
        const txResponse = await contractCreate.execute(client);
        console.log(`Txid: ${txResponse.transactionId}`);

        // Get the receipt of the transaction
        const receipt = await txResponse.getReceipt(client);

        // Get the new contract ID
        const newContractId = receipt.contractId;
        console.log('The new contract ID is ' + newContractId);

        const evmAddress = contractIdToEvmAddress(newContractId.shard, newContractId.realm, newContractId.num);

        console.log(`EVM address of the new contract is ${evmAddress}`);

        if (options.output) {
            const fs = require('fs');
            const output = {
                contractId: newContractId.toString(),
                evmAddress,
                contractName,
                sourceName: json.sourceName,
                deployedAt: new Date().toISOString()
            };
            fs.writeFileSync(options.output, JSON.stringify(output, null, 2));
            console.log(`Deployment info saved to ${options.output}`);
        }

    } catch (error) {
        console.error('Deployment failed:', error.message);
        process.exit(1);
    } finally {
        process.exit(0);
    }
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-hts-lib')
        .description('Deploy HTS library contract to Hedera')
        .addOption(
            new Option('--gas <gas>', 'gas limit for deployment')
                .default(300_000)
                .argParser((value) => parseInt(value, 10))
        )
        .addOption(
            new Option('--output <output>', 'output file path to save deployment info')
        )
        .action((options) => {
            deployHtsLib(null, options);
        });

	addBaseOptions(program);

  program.parse();
}

module.exports = {
    deployHtsLib,
    contractIdToEvmAddress
};
