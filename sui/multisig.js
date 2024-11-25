const { Command, Option } = require('commander');
const { fromB64 } = require('@mysten/bcs');
const { saveConfig, getChainConfig } = require('../common/utils');
const { loadConfig, printInfo, validateParameters, printWarn } = require('../common/utils');
const { getSignedTx, storeSignedTx } = require('../evm/sign-utils');
const { addBaseOptions, getWallet, getMultisig, signTransactionBlockBytes, broadcastSignature } = require('./utils');

async function initMultisigConfig(chain, options) {
    const { base64PublicKeys, threshold } = options;

    if (!base64PublicKeys) {
        throw new Error('Please provide public keys with --base64PublicKeys option');
    }

    if (!threshold) {
        throw new Error('Please provide threshold with --threshold option');
    }

    const uniqueKeys = new Set(base64PublicKeys);

    if (uniqueKeys.size !== base64PublicKeys.length) {
        throw new Error('Duplicate public keys found');
    }

    const schemeTypes = options.schemeTypes || Array(base64PublicKeys.length).fill('secp256k1');
    const weights = options.weights || Array(base64PublicKeys.length).fill(1);

    if (!options.schemeTypes) {
        printWarn('Scheme types not provided, defaulting to secp256k1');
    }

    if (!options.weights) {
        printWarn('Weights not provided, defaulting to 1');
    }

    if (base64PublicKeys.length !== weights.length) {
        throw new Error('Public keys and weights length mismatch');
    }

    if (base64PublicKeys.length !== schemeTypes.length) {
        throw new Error('Public keys and scheme types length mismatch');
    }

    const signers = base64PublicKeys.map((key, i) => ({
        publicKey: key,
        weight: parseInt(weights[i]),
        schemeType: options.schemeTypes[i],
    }));

    chain.multisig = {
        signers,
        threshold: parseInt(threshold),
    };

    printInfo('Saved multisig config');

    // To print in the separate lines with proper indentation
    printInfo(JSON.stringify(chain.multisig, null, 2));
}

async function signTx(keypair, client, options) {
    const txFileData = getSignedTx(options.txBlockPath);
    const txData = txFileData?.unsignedTx;

    validateParameters({ isNonEmptyString: { txData } });

    const encodedTxBytes = fromB64(txData);

    if (options.offline) {
        const { signature, publicKey } = await signTransactionBlockBytes(keypair, client, encodedTxBytes, options);
        return { ...txFileData, signedTx: signature, publicKey };
    }

    await signTransactionBlockBytes(keypair, client, encodedTxBytes, options);
    return {};
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
    const txData = fileData.unsignedTx;

    validateParameters({ isNonEmptyString: { txData } });

    const encodedTxBytes = fromB64(txData);
    const combinedSignatureBytes = fileData.signedTx;

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
    const txBytes = firstSignData.unsignedTx;

    for (const file of signatures) {
        const fileData = getSignedTx(file);

        if (fileData.unsignedTx !== txBytes) {
            throw new Error(`Transaction bytes mismatch with file [${file}]`);
        }

        signatureArray.push(fileData.signedTx);
    }

    const txBlockBytes = fromB64(txBytes);

    const combinedSignature = multiSigPublicKey.combinePartialSignatures(signatureArray);
    const isValid = await multiSigPublicKey.verifyTransaction(txBlockBytes, combinedSignature);

    if (!isValid) {
        throw new Error(`Verification failed for message [${txBytes}]`);
    }

    if (!options.offline) {
        const txResult = await broadcastSignature(client, txBlockBytes, combinedSignature);

        if (options.executeResultPath) {
            storeSignedTx(options.executeResultPath, txResult);
        }

        printInfo('Executed', txResult.digest);
    } else {
        const data = {
            message: firstSignData.message,
            signedTx: combinedSignature,
            status: 'PENDING',
            unsignedTx: txBytes,
        };
        return data;
    }
}

async function processCommand(chain, options) {
    const [keypair, client] = getWallet(chain, options);
    printInfo('Wallet Address', keypair.toSuiAddress());

    let fileData;

    switch (options.action) {
        case 'init': {
            fileData = await initMultisigConfig(chain, options);
            break;
        }

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
        let { signatureFilePath } = options;

        if (options.action === 'combine') {
            signatureFilePath = options.txBlockPath;
        }

        if (!signatureFilePath) {
            throw new Error('No filePath provided');
        }

        storeSignedTx(signatureFilePath, fileData);
        printInfo(`The signed signature is`, fileData.signedTx);
    }
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(chain, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('multisig').description('Script for multisig operators to sign, combine and execute data');

    addBaseOptions(program);

    program.addOption(new Option('--txBlockPath <file>', 'path to unsigned tx block'));
    program.addOption(new Option('--action <action>', 'action').choices(['sign', 'combine', 'execute', 'init']).makeOptionMandatory(true));
    program.addOption(new Option('--multisigKey <multisigKey>', 'multisig key').env('MULTISIG_KEY'));
    program.addOption(new Option('--signatures [files...]', 'array of signed transaction files'));
    program.addOption(new Option('--offline', 'run in offline mode'));
    program.addOption(new Option('--combinedSignPath <file>', 'combined signature file path'));
    program.addOption(new Option('--signatureFilePath <file>', 'signed signature will be stored'));
    program.addOption(new Option('--executeResultPath <file>', 'execute result will be stored'));

    // The following options are only used with the init action
    program.addOption(
        new Option('--base64PublicKeys [base64PublicKeys...]', 'An array of public keys to use for init the multisig address'),
    );
    program.addOption(
        new Option('--weights [weights...]', 'An array of weight for each base64 public key. The default value is 1 for each'),
    );
    program.addOption(
        new Option(
            '--schemeTypes [schemeTypes...]',
            'An array of scheme types for each base64 public key. The default value is secp256k1 for each',
        ),
    );
    program.addOption(new Option('--threshold <threshold>', 'threshold for multisig'));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
