'use strict';

const { loadConfig, printInfo, printWarn, printError, printHighlight, callAxelarscanApi, printDivider } = require('../common/index.js');
const { Command, Option } = require('commander');
const { addBaseOptions } = require('../common/cli-utils');

const { deriveAccounts } = require('./utils.js');

const { its } = require('./its.js');

const fs = require('fs');
const chalk = require('chalk');

const ITS_ACTION_INTERCHAIN_TRANSFER = 'interchain-transfer';
const ITS_ACTION_TOKEN_ADDRESS = 'interchain-token-address';

const writeTransactions = async (writing, transactions, stream) => {
    if (writing || transactions.length === 0) {
        return;
    }

    writing = true;

    printHighlight(`Writing ${transactions.length} transactions to file ${stream.path}`);

    const content = transactions.splice(0).join('\n').concat('\n');
    stream.write(content);

    writing = false;
};

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

const printStats = (txCount, elapsedTime, privateKeys) => {
    printDivider();
    printInfo('Txs count', txCount.toString());
    printInfo('Elapsed time (min)', elapsedTime / 60);
    printInfo('Tx per second', txCount / elapsedTime);
    printInfo('Private keys in queue to be processed', privateKeys.length);
    printDivider();
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
        privateKey,
        gasValue,
        output,
    } = options;

    const stream = fs.createWriteStream(output, { flags: 'w' });
    const transactions = [];
    let writing = false;

    const itsOptions = {
        chainNames: sourceChain,
        destinationChain,
        tokenId,
        destinationAddress,
        amount: transferAmount,
        gasValue: gasValue || (await estimateGas(config, options)),
        metadata: '0x',
        env,
        yes: true,
    };

    let privateKeys = [];
    if (mnemonic) {
        const accounts = await deriveAccounts(mnemonic, addressesToDerive);
        privateKeys = accounts.map((account) => account.privateKey);
    } else {
        privateKeys = [privateKey];
    }

    const startTime = performance.now();
    let elapsedTime = 0;
    let txCount = 0;
    let printWarning = true;

    const pendingPromises = new Map();
    let promiseCounter = 0;

    const writeInterval = setInterval(() => writeTransactions(writing, transactions, stream), 5000);

    do {
        const pk = privateKeys.shift();

        if (pk) {
            const promiseId = promiseCounter++;

            const promise = its(ITS_ACTION_INTERCHAIN_TRANSFER, [], { ...itsOptions, privateKey: pk })
                .then((txHash) => {
                    txCount++;

                    transactions.push(txHash);
                })
                .catch((error) => {
                    printError(`Error while executing transaction ${txCount + 1}`, error);
                })
                .finally(() => {
                    elapsedTime = (performance.now() - startTime) / 1000;

                    setTimeout(() => {
                        privateKeys.push(pk);
                        printWarning = true;
                    }, delay);

                    pendingPromises.delete(promiseId);

                    printStats(txCount, elapsedTime, privateKeys);
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
    await writeTransactions(writing, transactions, stream);
    stream.end();

    const endTime = performance.now();
    printInfo('Execution time (minutes)', (endTime - startTime) / 1000 / 60);
};

const appendToFile = (stream, content) => {
    stream.write(content.concat('\n'));
};

const handleFailedOrPendingTransaction = (failStream, pendingStream, txHash, result) => {
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

    // append to the output files instead of overwriting them if resuming from a specific transaction
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
            handleFailedOrPendingTransaction(failStream, pendingStream, txHash, first);
            failed++;
            continue;
        }

        const second = await callAxelarscanApi(config, 'gmp/searchGMP', {
            messageId,
        });

        const status = second?.data[0]?.status;
        if (status !== 'executed') {
            handleFailedOrPendingTransaction(failStream, pendingStream, txHash, second);
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
        .requiredOption('-s, --source-chain <sourceChain>', 'source chain')
        .requiredOption('-d, --destination-chain <destinationChain>', 'destination chain')
        .requiredOption('--destination-address <destinationAddress>', 'destination address')
        .requiredOption('--token-id <tokenId>', 'token id')
        .requiredOption('--transfer-amount <transferAmount>', 'transfer amount, e.g. 0.001')
        .option('--gas-value <gasValue>', 'gas value')
        .addOption(
            new Option('-t, --time <time>', 'time limit in minutes to run the test')
                .makeOptionMandatory(true)
                .argParser((value) => parseInt(value) * 60),
        )
        .addOption(new Option('--delay <delay>', 'delay in milliseconds between calls').default(10))
        .addOption(
            new Option(
                '--addresses-to-derive <addresses-to-derive>',
                'number of addresses to derive from mnemonic, used as source addresses to generate load in parallel',
            ).env('DERIVE_ACCOUNTS'),
        )
        .addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').env('MNEMONIC'))
        .addOption(new Option('-o, --output <output>', 'output file to save the transactions generated').default('/tmp/load-test.txt'))
        .hook('preAction', (command) => {
            const addressesToDerive = command.opts().addressesToDerive;
            const mnemonic = command.opts().mnemonic;

            if (addressesToDerive && !mnemonic) {
                throw new Error('Mnemonic is required when deriving addresses');
            }
        })
        .action((options) => {
            mainProcessor(startTest, options);
        });
    addBaseOptions(loadTestCmd, { ignoreChainNames: true });

    const verifyCmd = program
        .command('verify')
        .description('Verify the results of a load test')
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
    addBaseOptions(verifyCmd, { ignoreChainNames: true, ignorePrivateKey: true });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
