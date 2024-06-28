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

async function executeCombinedSignature(client, txBlockBytes, options) {
    const { combinedSignPath, txData } = options;

    if (options.offline) {
        throw new Error('Cannot execute in offline mode');
    }

    if (!combinedSignPath) {
        throw new Error('Invalid filePath provided');
    }

    const fileData = getSignedTx(combinedSignPath);

    if (fileData.message !== txData) {
        throw new Error(`Message mismatch with file [${combinedSignPath}]`);
    }

    const combinedSignatureBytes = fileData.signature;

    if (!combinedSignatureBytes) {
        throw new Error(`No signature specified in [${combinedSignPath}]`);
    }

    const txResult = await broadcastSignature(client, txBlockBytes, combinedSignatureBytes);
    printInfo('Transaction result', JSON.stringify(txResult));

    fileData.status = 'EXECUTED';
    storeSignedTx(combinedSignPath, fileData);
}

async function combineSignature(client, chain, txBlockBytes, options) {
    const { signatures, txData } = options;

    if (!signatures || signatures.length === 0) {
        throw new Error('FilePath is not provided in user info');
    }

    const multiSigPublicKey = await getMultisig(chain, options.multisigKey);
    const signatureArray = [];

    for (const file of signatures) {
        const fileData = getSignedTx(file);

        if (fileData.message !== txData) {
            throw new Error(`Message mismatch with file [${file}]`);
        }

        signatureArray.push(fileData.signature);
    }

    const combinedSignature = multiSigPublicKey.combinePartialSignatures(signatureArray);
    const isValid = await multiSigPublicKey.verifyTransactionBlock(txBlockBytes, combinedSignature);

    if (!isValid) {
        throw new Error(`Verification failed for message [${txData}]`);
    }

    if (!options.offline) {
        const txResult = await broadcastSignature(client, txBlockBytes, combinedSignature);
        printInfo('Transaction result', JSON.stringify(txResult));
    } else {
        const data = {
            signature: combinedSignature,
            status: 'PENDING',
        };
        return data;
    }
}

async function processCommand(chain, options) {
    const [keypair, client] = getWallet(chain, options);
    printInfo('Wallet Address', keypair.toSuiAddress());

    const txfileData = getSignedTx(options.txBlockPath);
    const txData = txfileData?.bytes;

    validateParameters({ isNonEmptyString: { txData } });

    options.txData = txData;
    const txBlockBytes = fromB64(txData);

    let fileData;

    switch (options.action) {
        case 'sign': {
            fileData = await signTx(keypair, client, txBlockBytes, options);
            break;
        }

        case 'combine': {
            fileData = await combineSignature(client, chain, txBlockBytes, options);
            break;
        }

        case 'execute': {
            await executeCombinedSignature(client, txBlockBytes, options);
            break;
        }

        default: {
            throw new Error(`Invalid action provided [${options.action}]`);
        }
    }

    if (options.offline) {
        const { signatureFilePath } = options;

        if (!signatureFilePath) {
            throw new Error('No filePath provided');
        }

        fileData.message = txData;
        storeSignedTx(signatureFilePath, fileData);
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

    program.addOption(new Option('--txBlockPath <file>', 'path to unsigned tx block').env('TX_FILE'));
    program.addOption(new Option('--action <action>', 'action').choices(['sign', 'combine', 'execute']).makeOptionMandatory(true));
    program.addOption(new Option('--multisigKey <multisigKey>', 'multisig key').env('MULTISIG_KEY'));
    program.addOption(new Option('--signatures [files...]', 'array of signed transaction files'));
    program.addOption(new Option('--offline', 'run in offline mode'));
    program.addOption(new Option('--combinedSignPath <file>', 'combined signature file path'));
    program.addOption(new Option('--signatureFilePath <file>', 'signed signature will be stored'));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
