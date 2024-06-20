const { Command, Option } = require('commander');
const { fromB64 } = require('@mysten/bcs');
const { addBaseOptions } = require('./cli-utils');
const { getWallet, getMultisig, signTransactionBlockBytes, broadcastSignature } = require('./sign-utils');
const { getSignedTx, storeSignedTx } = require('../evm/sign-utils');
const { loadSuiConfig } = require('./utils');
const { printInfo, validateParameters } = require('../evm/utils');

async function signTx(keypair, client, encodedTx, options) {
    return await signTransactionBlockBytes(keypair, client, encodedTx, options);
}

async function executeCombinedSingature(client, txBlockBytes, options) {
    const { combinedSignature, txData } = options;

    if (!combinedSignature) {
        throw new Error('Invalid filePath provided');
    }

    const fileData = getSignedTx(combinedSignature);

    if (fileData.message !== txData) {
        throw new Error(`Message mismatch with file [${combinedSignature}]`);
    }

    const combinedSignatureBytes = fileData.signature;

    if (!combinedSignatureBytes) {
        throw new Error(`No signature specified in [${combinedSignature}]`);
    }

    const txResult = await broadcastSignature(client, txBlockBytes, combinedSignatureBytes);
    printInfo('Transaction result', JSON.stringify(txResult));
}

async function combineSingature(client, chain, txBlockBytes, options) {
    const { signatures, txData } = options;

    if (!signatures || signatures.length === 0) {
        throw new Error('FilePath is not provided in user info');
    }

    const multiSigPublicKey = await getMultisig(chain, options.multisigKey);
    const singatures = [];

    for (const file of signatures) {
        const fileData = getSignedTx(file);

        if (fileData.message !== txData) {
            throw new Error(`Message mismatch with file [${file}]`);
        }

        singatures.push(fileData.signature);
    }

    const combinedSignature = multiSigPublicKey.combinePartialSignatures(singatures);

    const isValid = await multiSigPublicKey.verifyTransactionBlock(txBlockBytes, combinedSignature);

    if (!isValid) {
        throw new Error(`Verification failed for message [${txData}]`);
    }

    if (!options.offline) {
        const txResult = await broadcastSignature(client, txBlockBytes, combinedSignature);
        printInfo('Transaction result', JSON.stringify(txResult));
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

    const txfileData = getSignedTx(options.txData);
    const txData = txfileData?.bytes;

    if (!txData) {
        throw new Error(`Tx bytes not provided in [${txData}]`);
    }

    options.txData = txData;

    const txBlockBytes = fromB64(txData);

    validateParameters({ isNonEmptyString: { txData } });

    let fileData;

    switch (options.action) {
        case 'sign': {
            fileData = await signTx(keypair, client, txBlockBytes, options);
            break;
        }

        case 'combine': {
            fileData = await combineSingature(client, chain, txBlockBytes, options);
            break;
        }

        case 'execute': {
            await executeCombinedSingature(client, txBlockBytes, options);
            break;
        }

        default: {
            throw new Error(`Invalid action provided [${options.action}]`);
        }
    }

    if (options.offline && options.action !== 'execute') {
        const { txFile } = options;

        if (!txFile) {
            throw new Error('No filePath provided');
        }

        fileData.message = txData;
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

    program.name('multisig').description('Script for multisig operators to sign, combine and execute data');

    addBaseOptions(program);

    program.addOption(new Option('--txData <file>', 'file with tx data to be signed').env('TX_DATA'));
    program.addOption(new Option('--action <action>', 'action').choices(['sign', 'combine', 'execute']).makeOptionMandatory(true));
    program.addOption(new Option('--multisigKey <multisigKey>', 'multisig key to combine singature').env('MULTISIG_KEY'));
    program.addOption(new Option('--signatures [files...]', 'array of signed transaction files'));
    program.addOption(new Option('--offline', 'run in offline mode'));
    program.addOption(new Option('--combinedSignature <file>', 'file path to the combined signature'));
    program.addOption(new Option('--txFile <file>', 'file where the signed signature will be stored'));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
