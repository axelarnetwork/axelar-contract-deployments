const { Transaction } = require('@mysten/sui/transactions');
const { Command, Option } = require('commander');
const { loadConfig, validateParameters, getChainConfig } = require('../common/utils');
const { getWallet, printWalletInfo, addExtendedOptions, broadcast, saveGeneratedTx } = require('./utils');

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
        isKeccak256Hash: { objectId },
    });

    const tx = new Transaction();
    tx.transferObjects([`${objectId}`], tx.pure.address(recipient));

    if (options.offline) {
        const sender = options.sender || keypair.toSuiAddress();
        tx.setSender(sender);
        await saveGeneratedTx(tx, 'Transferred Object', client, options);
    } else {
        await broadcast(client, keypair, tx, 'Transferred Object');
    }
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(chain, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('transfer-object').description('Transfer object to recipient address');

    addExtendedOptions(program, { contractName: true });

    program.addOption(new Option('--objectId <objectId>', 'object id to be transferred'));
    program.addOption(new Option('--objectName <objectName>', 'object name to be transferred'));
    program.addOption(new Option('--recipient <recipient>', 'recipient to transfer object to').makeOptionMandatory(true));
    program.addOption(new Option('--sender <sender>', 'transaction sender'));
    program.addOption(new Option('--offline', 'store tx block for sign'));
    program.addOption(new Option('--txFilePath <file>', 'unsigned transaction will be stored'));

    program.action(async (options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
