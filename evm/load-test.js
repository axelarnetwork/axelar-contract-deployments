'use strict';

const { loadConfig, printInfo, printWarn, printHighlight, callAxelarscanApi } = require('../common/index.js');
const { Command, Option } = require('commander');
const { addBaseOptions } = require('../common/cli-utils.js');

const { deriveAccounts } = require('./utils.js');

const { its } = require('./its.js');

const fs = require('fs');
const chalk = require('chalk');

const ITS_ACTION_INTERCHAIN_TRANSFER = 'interchain-transfer';
const ITS_ACTION_TOKEN_ADDRESS = 'interchain-token-address';

let writing = false;
let transactions = [];
let stream = null;

const estimateGas = async (config, options) => {
    const { sourceChain, destinationChain, tokenId, privateKey, env } = options;

    const tokenAddress = await its(ITS_ACTION_TOKEN_ADDRESS, [tokenId], {
        chainNames: sourceChain,
        env,
        yes: true,
        privateKey,
    })[0];

    const gasFee = await callAxelarscanApi(config, 'gmp/estimateITSFee', {
        sourceChain,
        destinationChain,
        sourceTokenAddress: tokenAddress,
        event: 'InterchainTransfer',
    });

    return gasFee.toString();
};

const writeTransactions = async () => {
    if (writing || transactions.length === 0) {
        return;
    }

    writing = true;

    printHighlight(`Writing ${transactions.length} transactions to file ${stream.path}`);

    const content = transactions.splice(0).join('\n').concat('\n');
    stream.write(content);

    writing = false;
};

const startTest = async (config, options) => {
    const {
        time,
        delay,
        env,
        sourceChain,
        destinationChain,
        destinationAddress,
        tokenId,
        transferAmount,
        addressesToDerive,
        mnemonic,
        output,
    } = options;

    stream = fs.createWriteStream(output, { flags: 'w' });

    const args = [destinationChain, tokenId, destinationAddress, transferAmount];

    const itsOptions = {
        chainNames: sourceChain,
        gasValue: await estimateGas(config, options),
        metadata: '0x',
        env,
        yes: true,
    };

    const accounts = await deriveAccounts(mnemonic, addressesToDerive);
    const privateKeys = accounts.map((account) => account.privateKey);

    const startTime = performance.now();
    let elapsedTime = 0;
    let txCount = 0;
    let printWarning = true;

    const pendingPromises = new Map();
    let promiseCounter = 0;

    const writeInterval = setInterval(writeTransactions, 5000);

    do {
        const pk = privateKeys.shift();

        if (pk) {
            const promiseId = promiseCounter++;

            const promise = its(ITS_ACTION_INTERCHAIN_TRANSFER, args, { ...itsOptions, privateKey: pk })
                .then((txHash) => {
                    txCount++;

                    transactions.push(txHash);
                })
                .catch((error) => {
                    console.error('Error while running script:', error);
                })
                .finally(() => {
                    elapsedTime = (performance.now() - startTime) / 1000;

                    privateKeys.push(pk);
                    printWarning = true;

                    pendingPromises.delete(promiseId);

                    console.log('='.repeat(20).concat('\n'));
                    printInfo('Txs count', txCount.toString());
                    printInfo('Elapsed time (min)', elapsedTime / 60);
                    printInfo('Tx per second', txCount / elapsedTime);
                    printInfo('Private keys length', privateKeys.length);
                    console.log('='.repeat(20).concat('\n'));
                });

            pendingPromises.set(promiseId, promise);

            await new Promise((resolve) => setTimeout(resolve, delay));
        } else {
            if (printWarning) {
                printWarn('No more private keys to use, waiting for delay', delay);
                printWarning = false;
            }

            await new Promise((resolve) => setTimeout(resolve, delay));
        }
    } while (elapsedTime < time);

    if (pendingPromises.size > 0) {
        await Promise.all(pendingPromises.values());
    }

    clearInterval(writeInterval);
    await writeTransactions();
    stream.end();

    const endTime = performance.now();
    printInfo('Execution time (minutes)', (endTime - startTime) / 1000 / 60);
};

const appendToFile = (stream, content) => {
    stream.write(content.concat('\n'));
};

const handleIncompleteTransaction = (failStream, pendingStream, txHash, result) => {
    const status = result?.data[0]?.status;

    let message = status;
    let color = chalk.bgYellow;
    let stream = pendingStream;

    if (status === 'error') {
        message = message.concat(`: ${result?.data[0]?.error?.error?.message}`);
        color = chalk.bgRed;
        stream = failStream;
    }

    appendToFile(stream, `${txHash} : ${message}`);
    printInfo('Verification status', message, color);
};

const verify = async (config, options) => {
    const { inputFile, delay, failOutput, pendingOutput, successOutput, resumeFrom } = options;

    let transactions = fs.readFileSync(inputFile, 'UTF8').split('\n');

    if (transactions[transactions.length - 1] === '') {
        transactions = transactions.slice(0, -1);
    }

    // Get the total number of transactions before slicing the array when resuming from a specific transaction
    const totalTransactions = transactions.length;

    let streamFlags = 'w';
    if (resumeFrom > 0) {
        streamFlags = 'a';
        transactions = transactions.slice(resumeFrom);
    }

    const failStream = fs.createWriteStream(failOutput, { flags: streamFlags });
    const pendingStream = fs.createWriteStream(pendingOutput, { flags: streamFlags });
    const successStream = fs.createWriteStream(successOutput, { flags: streamFlags });

    let failed = 0;
    let successful = 0;

    for (const [index, line] of transactions.entries()) {
        if (!line || line.trim() === '') {
            continue; // Skip empty transaction hashes
        }

        // Extract the tx hash from the line in case it contains a message, e.g. "0x1234567890123456789012345678901234567890 : error: message"
        const txHash = line.split(':')[0].trim();

        printInfo(`Verifying transaction ${index + 1 + resumeFrom} of ${totalTransactions}`, txHash);

        const first = await callAxelarscanApi(config, 'gmp/searchGMP', {
            txHash: txHash,
        });

        const messageId = first?.data[0]?.callback?.id;
        if (!messageId) {
            handleIncompleteTransaction(failStream, pendingStream, txHash, first);
            failed++;
            continue;
        }

        const second = await callAxelarscanApi(config, 'gmp/searchGMP', {
            messageId,
        });

        const status = second?.data[0]?.status;
        if (status !== 'executed') {
            handleIncompleteTransaction(failStream, pendingStream, txHash, second);
            failed++;
            continue;
        }

        appendToFile(successStream, txHash);
        printInfo('Verification status', 'successful', chalk.bgGreen);
        successful++;
        await new Promise((resolve) => setTimeout(resolve, delay));
    }

    failStream.end();
    pendingStream.end();
    successStream.end();

    printInfo('Failed or incomplete transactions', failed);
    printInfo('Successful transactions', successful);
};

const mainProcessor = async (processor, options) => {
    const { env } = options;
    const config = loadConfig(env);

    await processor(config, options);
};

const programHandler = () => {
    const program = new Command();

    program.name('load-test').description('Load testing tools');

    const loadTestCmd = program
        .command('test')
        .description('Start a load test')
        .option('-s, --source-chain <sourceChain>', 'source chain')
        .option('-d, --destination-chain <destinationChain>', 'destination chain')
        .option('--destination-address <destinationAddress>', 'destination address')
        .option('--token-id <tokenId>', 'token id')
        .option('--transfer-amount <transferAmount>', 'transfer amount, e.g. 0.001')
        .addOption(new Option('-t, --time <time>', 'time limit in minutes to run the test').argParser((value) => parseInt(value) * 60))
        .addOption(new Option('--delay <delay>', 'delay in milliseconds between calls').default(10))
        .addOption(
            new Option(
                '--addresses-to-derive <addresses-to-derive>',
                'quantity of accounts to derive from mnemonic, used as source addresses to execute parallel transfers',
            ).env('DERIVE_ACCOUNTS'),
        )
        .addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'))
        .addOption(new Option('-o, --output <output>', 'output file to save the transactions generated').default('/tmp/load-test.txt'))
        .action((options) => {
            mainProcessor(startTest, options);
        });
    addBaseOptions(loadTestCmd, { ignoreChainNames: true });

    const verifyCmd = program
        .command('verify')
        .description('Verify a load test')
        .addOption(
            new Option(
                '--resume-from <resumeFrom>',
                'resume from transaction number (inclusive, one-based index), this will append to the output files instead of overwriting them',
            )
                .default(0)
                .argParser((value) => parseInt(value) - 1),
        )
        .addOption(new Option('--delay <delay>', 'delay in milliseconds between transaction verifications').default(100))
        .addOption(new Option('-i, --input-file <inputFile>', 'input file with transactions to verify').default('/tmp/load-test.txt'))
        .addOption(
            new Option('-f, --fail-output <failOutput>', 'output file to save the failed transactions').default('/tmp/load-test-fail.txt'),
        )
        .addOption(
            new Option('-p, --pending-output <pendingOutput>', 'output file to save the pending transactions').default(
                '/tmp/load-test-pending.txt',
            ),
        )
        .addOption(
            new Option('-s, --success-output <successOutput>', 'output file to save the successful transactions').default(
                '/tmp/load-test-success.txt',
            ),
        )
        .action((options) => {
            mainProcessor(verify, options);
        });
    addBaseOptions(verifyCmd, { ignoreChainNames: true });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
