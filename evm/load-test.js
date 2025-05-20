'use strict';

const { loadConfig, printInfo, printWarn, printHighlight, callAxelarscanApi } = require('../common/index.js');
const { Command, Option } = require('commander');
const { addBaseOptions } = require('../common/cli-utils.js');

const { httpPost, deriveAccounts } = require('./utils.js');

const { its } = require('./its.js');

const ethers = require('ethers');
const fs = require('fs');
const chalk = require('chalk');

const ITS_EXAMPLE_PAYLOAD =
        '0x0000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000047872706c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001800000000000000000000000000000000000000000000000000000000000000000ba5a21ca88ef6bba2bfff5088994f90e1077e2a1cc3dcc38bd261f00fce2824f00000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000de0b6b3a764000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000014ba76c6980428a0b10cfc5d8ccb61949677a6123300000000000000000000000000000000000000000000000000000000000000000000000000000000000000227277577142334d3352694c634c724c6d754e34524e5964594c507239544e384831430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000';
const ITS_ACTION = 'interchain-transfer';

let writing = false;
let transactions = [];
let stream = null;

const estimateGas = async (config, options) => {
    const { sourceChain, destinationChain, executionGasLimit, tokenId } = options;

    const gasFee = await callAxelarscanApi(config, 'gmp/estimateGasFee', {
        sourceChain,
        destinationChain,
        sourceTokenAddress: ethers.constants.AddressZero,
        gasLimit: executionGasLimit,
        executeData: ITS_EXAMPLE_PAYLOAD,
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
}

const startTest = async (config, options) => {
    const { time, delay, env, sourceChain, destinationChain, destinationAddress, tokenId, transferAmount, addressesToDerive, mnemonic, output } = options;

    stream = fs.createWriteStream(output, { flags: 'w' });

    const args = [
        destinationChain,
        tokenId,
        destinationAddress,
        transferAmount
    ];

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

            const promise = its(ITS_ACTION, args, { ...itsOptions, privateKey: pk })
                .then((txHash) => {
                    txCount++;

                    transactions.push(txHash);
                })
                .catch((error) => {
                    console.error('Error while running script:', error);
                })
                .finally(() => {
                    elapsedTime = (performance.now() - startTime) / 1000;

                    console.log('='.repeat(20).concat('\n'));
                    printInfo('Txs count', txCount.toString());
                    printInfo('Elapsed time (min)', elapsedTime / 60);
                    printInfo('Tx per second', txCount / elapsedTime);
                    printInfo('Private keys length', privateKeys.length);
                    console.log('='.repeat(20).concat('\n'));

                    privateKeys.push(pk);
                    printWarning = true;

                    pendingPromises.delete(promiseId);
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

const verify = async (config, options) => {
    const { inputFile, delay, failOutput, successOutput } = options;

    const failStream = fs.createWriteStream(failOutput, { flags: 'w' });
    const successStream = fs.createWriteStream(successOutput, { flags: 'w' });

    const transactions = fs.readFileSync(inputFile, 'UTF8').split('\n');

    let failed = 0;
    let successful = 0;

    for (const txHash of transactions) {
        if (!txHash || txHash.trim() === '') {
            continue; // Skip empty transaction hashes
        }

        printInfo('Verifying transaction', txHash);

        const first = await callAxelarscanApi(config, 'gmp/searchGMP', {
            txHash: txHash
        });

        const messageId = first?.data[0]?.callback?.id;
        if (!messageId) {
            printWarn('First GMP not complete', txHash);
            appendToFile(failStream, txHash);
            failed++;
            continue;
        }

        const second = await callAxelarscanApi(config, 'gmp/searchGMP', {
            messageId
        });

        const status = second?.data[0]?.status;
        if (status !== 'executed') {
            printWarn('Second GMP not complete', txHash);
            appendToFile(failStream, txHash);
            failed++;
            continue;
        }

        appendToFile(successStream, txHash);
        successful++;
        await new Promise((resolve) => setTimeout(resolve, delay));
    }

    failStream.end();
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

    program.name('load-test')
        .description('Load testing tools')

    const loadTestCmd = program
        .command('test')
        .description('Start a load test')
        .option('-s, --source-chain <sourceChain>', 'source chain')
        .option('-d, --destination-chain <destinationChain>', 'destination chain')
        .option('--destination-address <destinationAddress>', 'destination address')
        .option('--token-id <tokenId>', 'token id')
        .option('--transfer-amount <transferAmount>', 'transfer amount, e.g. 0.001')
        .option('--executionGasLimit <executionGasLimit>', 'execution gas limit')
        .addOption(new Option('-t, --time <time>', 'time limit in minutes to run the test').argParser((value) => parseInt(value) * 60))
        .addOption(new Option('--delay <delay>', 'delay in milliseconds between calls').default(10))
        .addOption(new Option('--addresses-to-derive <addresses-to-derive>', 'quantity of accounts to derive from mnemonic, used as source addresses to execute parallel transfers').env('DERIVE_ACCOUNTS'))
        .addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'))
        .addOption(new Option('-o, --output <output>', 'output file to save the transactions generated').default('/tmp/load-test.txt'))
        .action((options) => {
            mainProcessor(startTest, options);
        });
    addBaseOptions(loadTestCmd, { ignoreChainNames: true });

    const verifyCmd = program
        .command('verify')
        .description('Verify a load test')
        .addOption(new Option('--delay <delay>', 'delay in milliseconds between transaction verifications').default(100))
        .addOption(new Option('-i, --input-file <inputFile>', 'input file with transactions to verify').default('/tmp/load-test.txt'))
        .addOption(new Option('-f, --fail-output <failOutput>', 'output file to save the failed transactions').default('/tmp/load-test-fail.txt'))
        .addOption(new Option('-s, --success-output <successOutput>', 'output file to save the successful transactions').default('/tmp/load-test-success.txt'))
        .action((options) => {
            mainProcessor(verify, options);
        });
    addBaseOptions(verifyCmd, { ignoreChainNames: true });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
