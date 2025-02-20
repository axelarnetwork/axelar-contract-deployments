const { Transaction } = require('@mysten/sui/transactions');
const { Command, Option } = require('commander');
const { loadConfig, validateParameters, getChainConfig, printInfo } = require('../common/utils');
const { getWallet, printWalletInfo, addExtendedOptions, broadcast, paginateAll } = require('./utils');

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

    await broadcast(client, keypair, tx, 'Transferred Object');
}

async function listBagContents(chain, options, args) {
    const [objectId, fieldName] = args;
    const [client] = getWallet(chain, options);

    printInfo('Object Id', objectId);
    const objectDetails = await client.getObject({
        id: objectId,
        options: {
            showContent: true,
        },
    });

    const bagId = objectDetails.data.content.fields.value.fields[fieldName].fields.id.id;

    printInfo(`${fieldName} Id`, bagId);

    const result = await paginateAll(client, 'getDynamicFields', {
        parentId: bagId,
        name: 'unregistered_coins',
    });

    printInfo('Contents Length', result.length);
    const token = result.find((item) =>
        item.objectType.includes(`0xc3f5cf87e7dad927ac7286b626ab607ace5df519a1dc6b15aac8af11f4cc5848::bbb::BBB`),
    );
    console.log('Token content', token);

    const result2 = await client.getObject({
        id: token.objectId,
        options: {
            showContent: true,
        },
    });

    console.log('Token Fields', result2.data.content.fields.value.fields.coin_info.fields);
}

async function mainProcessor(options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(chain, options, args);
}

if (require.main === module) {
    const program = new Command();

    program.name('object').description('Object operations');

    const transferObjectCommand = new Command('transfer').description('Transfer object to recipient address');

    addExtendedOptions(transferObjectCommand, { contractName: true });

    transferObjectCommand.addOption(new Option('--objectId <objectId>', 'object id to be transferred'));
    transferObjectCommand.addOption(new Option('--objectName <objectName>', 'object name to be transferred'));
    transferObjectCommand.addOption(new Option('--recipient <recipient>', 'recipient to transfer object to').makeOptionMandatory(true));
    transferObjectCommand.action((options) => {
        mainProcessor(options, [], processCommand);
    });

    const listBagItemsCommand = new Command('list')
        .description('List bag items')
        .command('list <bagId> <fieldName>')
        .action((bagId, fieldName, options) => {
            mainProcessor(options, [bagId, fieldName], listBagContents);
        });

    addExtendedOptions(listBagItemsCommand, { contractName: true });

    program.addCommand(transferObjectCommand);
    program.addCommand(listBagItemsCommand);

    program.parse();
}
