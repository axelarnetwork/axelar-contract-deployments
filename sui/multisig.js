const { Command, Option } = require('commander');
const { fromB64 } = require('@mysten/bcs');
const { saveConfig, getChainConfig } = require('../common/utils');
const { loadConfig, printInfo, validateParameters, printWarn } = require('../common/utils');
const { getSignedTx, storeSignedTx } = require('../evm/sign-utils');
const { addBaseOptions, getWallet, getMultisig, signTransactionBlockBytes, broadcastSignature, getSuiClient } = require('./utils');

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

async function signTx(chain, options) {
    const [keypair, client] = getWallet(chain, options);

    printInfo('Wallet Address', keypair.toSuiAddress());

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

async function executeCombinedSignature(chain, options) {
    const client = getSuiClient(chain, options.rpc);
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

async function combineSignature(chain, options) {
    const client = getSuiClient(chain);
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

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const fileData = await processor(chain, options);
    saveConfig(config, options.env);

    if (options.offline) {
        const { signatureFilePath } = options;

        if (!signatureFilePath) {
            throw new Error('No filePath provided');
        }

        storeSignedTx(signatureFilePath, fileData);
        printInfo(`The signed signature is`, fileData.signedTx);
    }
}

if (require.main === module) {
    const program = new Command('multisig').description('Script for multisig operators to sign, combine and execute data');

    const initCmd = new Command('init').description('Add a sui Multisig to config');
    const signCmd = new Command('sign').description('Sign via Multisig signer');
    const combineCmd = new Command('combine').description('Combine signatures for a Multisig');
    const executeCmd = new Command('execute').description('Execute signatures for a Multisig');

    initCmd.addOption(
        new Option('--base64PublicKeys [base64PublicKeys...]', 'An array of public keys to use for init the multisig address'),
    );
    initCmd.addOption(
        new Option('--weights [weights...]', 'An array of weight for each base64 public key. The default value is 1 for each'),
    );
    initCmd.addOption(
        new Option(
            '--schemeTypes [schemeTypes...]',
            'An array of scheme types for each base64 public key. The default value is secp256k1 for each',
        ),
    );
    initCmd.addOption(new Option('--threshold <threshold>', 'threshold for multisig'));
    initCmd.action(async (options) => {
        await mainProcessor(options, initMultisigConfig);
    });

    signCmd.addOption(new Option('--txBlockPath <file>', 'path to unsigned tx block'));
    signCmd.addOption(new Option('--offline', 'run in offline mode'));
    signCmd.addOption(new Option('--signatureFilePath <file>', 'signed signature will be stored'));
    signCmd.action(async (options) => {
        await mainProcessor(options, signTx);
    });

    combineCmd.addOption(new Option('--txBlockPath <file>', 'path to unsigned tx block'));
    combineCmd.addOption(new Option('--offline', 'run in offline mode'));
    combineCmd.addOption(new Option('--signatures [files...]', 'array of signed transaction files'));
    combineCmd.addOption(new Option('--executeResultPath <file>', 'execute result will be stored'));
    combineCmd.addOption(new Option('--multisigKey <multisigKey>', 'multisig key').env('MULTISIG_KEY'));
    combineCmd.addOption(new Option('--signatureFilePath <file>', 'signed signature will be stored'));
    combineCmd.action(async (options) => {
        await mainProcessor(options, combineSignature);
    });

    executeCmd.addOption(new Option('--combinedSignPath <file>', 'combined signature file path'));
    executeCmd.addOption(new Option('-r, --rpc <rpc>', 'The custom rpc'));
    executeCmd.action(async (options) => {
        await mainProcessor(options, executeCombinedSignature);
    });

    const cmdOptions = { ignorePrivateKey: true };

    addBaseOptions(initCmd, cmdOptions);
    addBaseOptions(signCmd);
    addBaseOptions(combineCmd, cmdOptions);
    addBaseOptions(executeCmd, cmdOptions);

    program.addCommand(initCmd);
    program.addCommand(signCmd);
    program.addCommand(combineCmd);
    program.addCommand(executeCmd);

    program.parse();
}
