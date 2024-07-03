const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { Command, Option } = require('commander');
const { printInfo, validateParameters } = require('../evm/utils');
const { addExtendedOptions } = require('./cli-utils');
const { getWallet, printWalletInfo } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

async function processCommand(chain, options) {
    const [keypair, client] = getWallet(chain, options);
    await printWalletInfo(keypair, client, chain, options);
    const recipient = options.recipient;

    validateParameters({
        isKeccak256Hash: { recipient },
    });

    let objectId;

    if (options.objectId) {
        objectId = options.objectId;
    } else if (options.contractName && options.objectName) {
        const { contractName, objectName } = options;

        validateParameters({
            isString: { contractName, objectName },
        });

        const contractsData = chain?.contracts;
        const contractObject = contractsData?.[contractName];
        const objectsData = contractObject?.objects;
        objectId = objectsData?.[objectName];
    } else {
        throw new Error('Provide object id or contract name with object name');
    }

    validateParameters({
        isNonEmptyString: { objectId },
        isKeccak256Hash: { objectId },
    });

    const tx = new TransactionBlock();
    tx.transferObjects([`${objectId}`], tx.pure(recipient));

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

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);
    await processor(config.sui, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('transfer-object').description('Transfer object to recipient address');

    addExtendedOptions(program, { contractName: true });

    program.addOption(new Option('--objectId <objectId>', 'object id to be transferred'));
    program.addOption(new Option('--objectName <objectName>', 'object name to be transferred'));
    program.addOption(new Option('--recipient <recipient>', 'recipient to transfer object to').makeOptionMandatory(true));

    program.action(async (options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
