const { Command, Option } = require('commander');
const { fromB64 } = require('@mysten/bcs');
const { addBaseOptions } = require('./cli-utils');
const { getWallet, getMultisig, signTransactionBlockBytes, broadcastSignature } = require('./sign-utils');
const { getSignedTx, storeSignedTx } = require('../evm/sign-utils');
const { loadSuiConfig } = require('./utils');
const { printInfo, validateParameters } = require('../evm/utils');

async function signTx(keypair, client, options) {
    const txFileData = getSignedTx(options.txBlockPath);
    const txData = txFileData?.bytes;

    validateParameters({ isNonEmptyString: { txData } });

    const encodedTxBytes = fromB64(txData);

    const { signature, publicKey } = await signTransactionBlockBytes(keypair, client, encodedTxBytes, options);
    return { signature, publicKey, txBytes: txData };
}

async function executeCombinedSignature(client, options) {
    const { combinedSignPath } = options;

    if (options.offline) {
        throw new Error('Cannot execute in offline mode');
    }

    if (!combinedSignPath) {
        throw new Error('Invalid filePath provided');
    }

    const fileData = getSignedTx(combinedSignPath);
    const txData = fileData.txBytes;

    validateParameters({ isNonEmptyString: { txData } });

    const encodedTxBytes = fromB64(txData);
    const combinedSignatureBytes = fileData.signature;

    if (!combinedSignatureBytes) {
        throw new Error(`No signature specified in [${combinedSignPath}]`);
    }

    const txResult = await broadcastSignature(client, encodedTxBytes, combinedSignatureBytes);
    printInfo('Transaction result', JSON.stringify(txResult));

    fileData.status = 'EXECUTED';
    storeSignedTx(combinedSignPath, fileData);
}

async function combineSignature(client, chain, options) {
    const { signatures } = options;

    if (!signatures || signatures.length === 0) {
        throw new Error('FilePath is not provided in user info');
    }

    const multiSigPublicKey = await getMultisig(chain, options.multisigKey);
    const signatureArray = [];

    const firstSignData = getSignedTx(signatures[0]);
    const txBytes = firstSignData.txBytes;

    for (const file of signatures) {
        const fileData = getSignedTx(file);

        if (fileData.txBytes !== txBytes) {
            throw new Error(`Transaction bytes mismatch with file [${file}]`);
        }

        signatureArray.push(fileData.signature);
    }

    const txBlockBytes = fromB64(txBytes);

    const combinedSignature = multiSigPublicKey.combinePartialSignatures(signatureArray);
    const isValid = await multiSigPublicKey.verifyTransactionBlock(txBlockBytes, combinedSignature);

    if (!isValid) {
        throw new Error(`Verification failed for message [${txBytes}]`);
    }

    if (!options.offline) {
        const txResult = await broadcastSignature(client, txBlockBytes, combinedSignature);
        printInfo('Transaction result', JSON.stringify(txResult));
    } else {
        const data = {
            signature: combinedSignature,
            status: 'PENDING',
            txBytes,
        };
        return data;
    }
}

async function processCommand(chain, options) {
    const [keypair, client] = getWallet(chain, options);
    printInfo('Wallet Address', keypair.toSuiAddress());

    let fileData;

    switch (options.action) {
        case 'sign': {
            fileData = await signTx(keypair, client, options);
            break;
        }

        case 'combine': {
            fileData = await combineSignature(client, chain, options);
            break;
        }

        case 'execute': {
            await executeCombinedSignature(client, options);
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

    program.addOption(new Option('--txBlockPath <file>', 'path to unsigned tx block'));
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
