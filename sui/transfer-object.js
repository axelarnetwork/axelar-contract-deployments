const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { Command, Option } = require('commander');
const { loadConfig, printInfo, isKeccak256Hash, isString } = require('../evm/utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

async function transferCap(options) {
    const config = loadSuiConfig(options.env);
    const [keypair, client] = getWallet(config.sui, options);
    const tx = new TransactionBlock();

    tx.transferObjects([`${options.objectId}`], tx.pure(options.reciepent));

    const result = await client.signAndExecuteTransactionBlock({
        transactionBlock: tx,
        signer: keypair,
        options: {
            showObjectChanges: true,
            showBalanceChanges: true,
            showEvents: true,
        },
    });
    printInfo('Transaction result', JSON.stringify(result));
}

async function getConfigAndTransferObject(options) {
    const objectId = loadConfig(options.env).sui.contracts[`${options.contractName}`].objects[`${options.objectName}`];
    options.objectId = objectId;
    await transferCap(options);
}

async function main(options) {
    if (!isKeccak256Hash(options.reciepent)) {
        throw new Error(`Invalid reciepent [${options.reciepent}]`);
    }

    if (options.objectId) {
        if (!isKeccak256Hash(options.objectId)) {
            throw new Error(`Invalid object Id [${options.objectId}]`);
        }

        await transferCap(options);
    } else if (options.contractName && options.objectName) {
        if (!isString(options.contractName)) {
            throw new Error(`Invalid contract name [${options.contractName}]`);
        }

        if (!isString(options.objectName)) {
            throw new Error(`Invalid object name [${options.objectName}]`);
        }

        await getConfigAndTransferObject(options);
    } else {
        throw new Error('provide object id or contract name with object name');
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('transfer-object').description('Transfer object to recipient address');

    addBaseOptions(program);

    program.addOption(new Option('--objectId <objectId>', 'object id to be transfered').env('SUI_OBJECT_ID'));

    program.addOption(new Option('--contractName <contractName>', 'contract name').env('CONTRACT_NAME'));

    program.addOption(new Option('--objectName <objectName>', 'object name to be transfered').env('SUI_OBJECT_NAME'));

    program.addOption(
        new Option('--reciepent <reciepentAddress>', 'reciepent to transfer object to').makeOptionMandatory(true).env('RECIEPENT_ADDRESS'),
    );

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
}
