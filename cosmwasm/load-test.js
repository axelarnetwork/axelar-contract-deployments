'use strict';

const { prepareClient, prepareWallet, initContractConfig, executeContractMultiple, printBalance } = require('./utils');
const { loadConfig, printInfo, printWarn, printError } = require('../common');
const { Command, Option } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const { main: its } = require('../evm/its');

const crypto = require('crypto');

const xrplMsg = () => {
    const randomHash = crypto.randomBytes(32).toString('hex');

    return {
        verify_messages: [
            {
                call_contract_message: {
                    tx_id: 'e15349baff6a1add31d3d6a87ad59ce14e97234f2d125a482ae14e69c1a7351e',
                    source_address: 'rGAbJZEzU6WaYv5y1LfyN7LBBcQJ3TxsKC',
                    destination_chain: 'some-chain',
                    destination_address: '0xa1CdBFdcCed95910DD2496BC9711F183C204cBf2',
                    payload_hash: randomHash,
                    gas_fee_amount: { drops: 1700000 },
                },
            },
        ],
    };
};

const xrplVerifierBatch = async (client, wallet, config, options) => {
    const { time, batch, delay } = options;
    let pollsCount = 0;

    const startTime = performance.now();
    let elapsedTime = 0;

    while (elapsedTime < time * 1000) {
        const msgs = [];
        const pollIds = [];

        for (let i = 0; i < batch; i++) {
            msgs.push(xrplMsg());
        }

        let result;
        let retries = 0;

        while (retries < 5) {
            try {
                result = await executeContractMultiple(client, wallet, config, options, msgs);
                break;
            } catch (error) {
                retries++;

                printError('Failed to execute contract', error);
                printWarn(`Retrying [${retries}]...`);

                await new Promise((resolve) => setTimeout(resolve, 500));
            }
        }

        if (!result) {
            throw new Error('Too many retries');
        }

        result.events
            .filter(({ type }) => type === 'wasm-messages_poll_started')
            .forEach((event) => {
                const pollId = event.attributes.find(({ key }) => key === 'poll_id').value;
                pollIds.push(parseInt(JSON.parse(pollId)));
            });

        pollsCount += pollIds.length;
        elapsedTime = (performance.now() - startTime) / 1000;

        printInfo('Transaction', result.transactionHash);
        printInfo('Poll IDs', pollIds);
        printInfo('Poll count', pollsCount);
        printInfo('Elapsed time (min)', elapsedTime / 60);
        printInfo('Polls per second', pollsCount / elapsedTime);
        await printBalance(client, wallet, config);
        console.log('='.repeat(20));

        await new Promise((resolve) => setTimeout(resolve, delay));
    }

    const endTime = performance.now();
    printInfo('Execution time (ms)', endTime - startTime);
};

const xrpl = async (client, wallet, config, options) => {
    const { time, delay, privateKeys, env } = options;
    const action = 'interchain-transfer';
    const args = [
        'xrpl', // destination chain
        '0xba5a21ca88ef6bba2bfff5088994f90e1077e2a1cc3dcc38bd261f00fce2824f', // token ID
        '0x7277577142334d3352694c634c724c6d754e34524e5964594c507239544e38483143', // destination address
        '1000000000000', // amount
    ];

    const itsOptions = {
        chainNames: 'xrpl-evm',
        gasValue: '278789857820065200',
        metadata: '0x',
        env,
        yes: true,
    };

    const startTime = performance.now();
    let elapsedTime = 0;
    let txCount = 0;

    while (elapsedTime < time) {
        const results = await Promise.allSettled(
            privateKeys.map((pk) => {
                return its(action, args, { ...itsOptions, privateKey: pk });
            }),
        );

        const successCount = results.filter((result) => result.status === 'fulfilled').length;
        txCount += successCount;

        elapsedTime = (performance.now() - startTime) / 1000;

        console.log('='.repeat(20));
        printInfo('Txs count', txCount.toString());
        printInfo('Elapsed time (min)', elapsedTime / 60);
        printInfo('Tx per second', txCount / elapsedTime);

        await new Promise((resolve) => setTimeout(resolve, delay));
    }

    const endTime = performance.now();
    printInfo('Execution time (ms)', endTime - startTime);
};

const mainProcessor = async (processor, options) => {
    const { env } = options;
    const config = loadConfig(env);

    initContractConfig(config, options);

    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);

    await processor(client, wallet, config, options);
};

const programHandler = () => {
    const program = new Command();

    program.name('load-test').description('Load testing');

    const verifierBatchCmd = program
        .command('xrpl-verifier-batch')
        .description('Stress test the XRPL voting verifier')
        .option('-t, --time <time>', 'time limit in seconds to run the test')
        .option('-b, --batch <batch>', 'batch size per iteration')
        .option('-d, --delay <delay>', 'delay in milliseconds between batches')
        .action((options) => {
            mainProcessor(xrplVerifierBatch, options);
        });
    addAmplifierOptions(verifierBatchCmd, { contractOptions: true });

    const xrplCmd = program
        .command('xrpl')
        .description('Stress test full XRPL flow')
        .option('-t, --time <time>', 'time limit in seconds to run the test')
        .option('-d, --delay <delay>', 'delay in milliseconds between batches')
        .addOption(
            new Option('-p, --privateKeys <privateKeys>', 'comma separated list of private keys')
                .env('PRIVATE_KEYS')
                .argParser((pks) => pks.split(',').map((pk) => pk.trim())),
        )
        .action((options) => {
            mainProcessor(xrpl, options);
        });
    addAmplifierOptions(xrplCmd, {});

    program.parse();
};

if (require.main === module) {
    programHandler();
}
