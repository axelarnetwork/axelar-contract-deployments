'use strict';

const { loadConfig, printInfo, printWarn } = require('../common/index.js');
const { Command, Option } = require('commander');
const { addBaseOptions } = require('../common/cli-utils.js');

const { getWallet } = require('./sign-utils.js');
const { httpPost, getContractJSON, deriveAccounts } = require('./utils.js');

const { main: its } = require('./its.js');

const ethers = require('ethers');

const ITS_EXAMPLE_PAYLOAD =
        '0x0000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000047872706c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001800000000000000000000000000000000000000000000000000000000000000000ba5a21ca88ef6bba2bfff5088994f90e1077e2a1cc3dcc38bd261f00fce2824f00000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000de0b6b3a764000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000014ba76c6980428a0b10cfc5d8ccb61949677a6123300000000000000000000000000000000000000000000000000000000000000000000000000000000000000227277577142334d3352694c634c724c6d754e34524e5964594c507239544e384831430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000';
const ITS_ACTION = 'interchain-transfer';

const estimateGas = async (config, options) => {
    const { sourceChain, destinationChain, executionGasLimit } = options;
    const api = config.axelar.axelarscanApi;

    const gasFee = await httpPost(`${api}/gmp/estimateGasFee`, {
        sourceChain,
        destinationChain,
        sourceTokenAddress: ethers.constants.AddressZero,
        gasLimit: executionGasLimit,
        executeData: ITS_EXAMPLE_PAYLOAD,
    });

    return gasFee.toString();
};

const start = async (config, options) => {
    const { time, delay, env, sourceChain, destinationChain, destinationAddress, tokenId, transferAmount, mnemonic } = options;

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

    const accounts = await deriveAccounts(mnemonic, 1);
    const privateKeys = accounts.map((account) => account.privateKey);

    const startTime = performance.now();
    let elapsedTime = 0;
    let txCount = 0;
    let printWarning = true;

    while (elapsedTime < time) {
        const pk = privateKeys.shift();

        if (pk) {
            its(ITS_ACTION, args, { ...itsOptions, privateKey: pk })
                .then(() => {
                    txCount++;
                })
                .catch((error) => {
                    console.error('Error while running script:', error);
                })
                .finally(() => {
                    elapsedTime = (performance.now() - startTime) / 1000;

                    console.log('='.repeat(20));
                    printInfo('Txs count', txCount.toString());
                    printInfo('Elapsed time (min)', elapsedTime / 60);
                    printInfo('Tx per second', txCount / elapsedTime);
                    printInfo('Private keys length', privateKeys.length);
                    console.log('='.repeat(20));

                    privateKeys.push(pk);
                    printWarning = true;
                });

            await new Promise((resolve) => setTimeout(resolve, delay));
        } else {
            if (printWarning) {
                printWarn('No more private keys to use, waiting for delay', delay);
                printWarning = false;
            }

            await new Promise((resolve) => setTimeout(resolve, delay));
        }
    }

    const endTime = performance.now();
    printInfo('Execution time (ms)', endTime - startTime);
};

const mainProcessor = async (processor, options) => {
    const { env } = options;
    const config = loadConfig(env);

    await processor(config, options);
};

const programHandler = () => {
    const program = new Command();

    program.name('load-test')
        .description('Load testing')
        .option('-t, --time <time>', 'time limit in seconds to run the test')
        .option('-d, --delay <delay>', 'delay in milliseconds between calls')
        .option('-s, --source-chain <sourceChain>', 'source chain')
        .option('-d, --destination-chain <destinationChain>', 'destination chain')
        .option('--destination-address <destinationAddress>', 'destination address')
        .option('--token-id <tokenId>', 'token id')
        .option('--transfer-amount <transferAmount>', 'transfer amount')
        .option('--native-amount <nativeAmount>', 'native amount to fund accounts with if min-native-funds is not met')
        .option('--token-amount <tokenAmount>', 'token amount to fund accounts with if min-token-funds is not met')
        .option('--min-native-funds <minNativeFunds>', 'minimum native funds required')
        .option('--min-token-funds <minTokenFunds>', 'minimum token funds required')
        .option('--executionGasLimit <executionGasLimit>', 'execution gas limit')
        .addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'))
        .action((options) => {
            mainProcessor(start, options);
        });
    addBaseOptions(program, { ignoreChainNames: true });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
