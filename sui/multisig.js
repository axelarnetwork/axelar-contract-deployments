const { Command, Option } = require('commander');
const { fromB64 } = require('@mysten/bcs');
const { addBaseOptions } = require('./cli-utils');
const { getWallet, getMultisig, signTransactionBlockBytes, broadcastSignature } = require('./sign-utils');
const { getSignedTx, storeSignedTx } = require('../evm/sign-utils');
const { loadSuiConfig } = require('./utils');
const { printInfo, validateParameters } = require('../evm/utils');

async function signMessage(keypair, client, encodedMessage, options) {
    return await signTransactionBlockBytes(keypair, client, encodedMessage, options);
}

async function executeCombinedSingature(client, encodedMessage, options) {
    const { combinedSignature, message } = options;
    console.log(combinedSignature);

    if (!combinedSignature) {
        throw new Error('Invalid filePath provided');
    }

    const fileData = getSignedTx(combinedSignature);

    if (fileData.message !== message) {
        throw new Error(`Message mismatch with this file path [${combinedSignature}]`);
    }

    const singatures = fileData.singatures;

    if (!singatures || singatures.length === 0) {
        throw new Error(`Message mismatch with this file path [${combinedSignature}]`);
    }

    await broadcastSignature(client, encodedMessage, combinedSignature);
}

async function combineSingature(client, chain, encodedMessage, options) {
    const { signatures, message } = options;

    if (!signatures || signatures.length === 0) {
        throw new Error('FilePath is not provided in user info');
    }

    const multiSigPublicKey = await getMultisig(chain, options.multisigKey);
    const singatures = [];

    for (const file of signatures) {
        const fileData = getSignedTx(file);

        if (fileData.message !== message) {
            throw new Error(`Message mismatch with this file path [${file}]`);
        }

        singatures.push(fileData.signature);
    }

    const combinedSignature = multiSigPublicKey.combinePartialSignatures(singatures);

    const isValid = await multiSigPublicKey.verifyTransactionBlock(encodedMessage, combinedSignature);

    if (!isValid) {
        throw new Error(`Verification failed for message [${message}]`);
    }

    if (!options.offline) {
        await broadcastSignature(client, encodedMessage, combinedSignature);
    }

    const data = {
        signature: combinedSignature,
        status: 'PENDING',
    };

    return data;
}

async function processCommand(chain, options) {
    const [keypair, client] = getWallet(chain, options);
    printInfo('Wallet Address', keypair.toSuiAddress());

    const message = options.message;
    const encodedMessage = fromB64(message);

    validateParameters({ isNonEmptyString: { message } });

    let fileData;

    switch (options.action) {
        case 'sign': {
            fileData = await signMessage(keypair, client, encodedMessage, options);
            break;
        }

        case 'combine': {
            fileData = await combineSingature(client, chain, encodedMessage, options);
            break;
        }

        case 'execute': {
            await executeCombinedSingature(client, encodedMessage, options);
            break;
        }

        default: {
            throw new Error(`Invalid action provided [${options.action}]`);
        }
    }

    if (options.offline) {
        const { txFile } = options;

        if (!txFile) {
            throw new Error('Invalid filePath provided');
        }

        fileData.message = message;
        storeSignedTx(txFile, fileData);
        printInfo(`The signed signature is`, fileData.signature);
    }
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);
    await processor(config.sui, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('multisig').description('sign a message from the user key');

    addBaseOptions(program);

    program.addOption(new Option('-m, --message <message>', 'The message to be signed').makeOptionMandatory(true).env('MESSAGE'));
    program.addOption(new Option('--action <action>', 'signing action').choices(['sign', 'combine', 'execute']).makeOptionMandatory(true));
    program.addOption(new Option('--multisigKey <multisigKey>', 'Multisig key to combine singature').env('MULTISIG_KEY'));
    program.addOption(new Option('--signatures [files...]', 'The file where the signed tx are stored'));
    program.addOption(new Option('--offline', 'Run in offline mode'));
    program.addOption(new Option('--combinedSignature <file>', 'The file where the combined signature is stored'));
    program.addOption(new Option('--txFile <file>', 'The file where the signed signature will be store'));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
