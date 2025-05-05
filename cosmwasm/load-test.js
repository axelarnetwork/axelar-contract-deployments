'use strict';

const { prepareClient, prepareWallet, initContractConfig, executeContractMultiple, printBalance } = require('./utils');
const { loadConfig, printInfo } = require('../common');
const { Command } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

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

        const result = await executeContractMultiple(client, wallet, config, options, msgs);

        result.events
            .filter(({ type }) => type === 'wasm-messages_poll_started')
            .forEach((event) => {
                const pollId = event.attributes.find(({ key }) => key === 'poll_id').value;
                pollIds.push(parseInt(JSON.parse(pollId)));
            });

        pollsCount += pollIds.length;
        elapsedTime = performance.now() - startTime;

        printInfo('Transaction', result.transactionHash);
        printInfo('Poll IDs', pollIds);
        printInfo('Poll count', pollsCount);
        printInfo('Elapsed time (ms)', elapsedTime);
        printInfo('Polls per second', pollsCount / (elapsedTime / 1000));
        await printBalance(client, wallet, config);
        console.log('='.repeat(20));

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
        .description('Load test voting verifier')
        .option('-t, --time <time>', 'time limit in seconds to run the test')
        .option('-b, --batch <batch>', 'batch size per iteration')
        .option('-d, --delay <delay>', 'delay in milliseconds between calls')
        .action((options) => {
            mainProcessor(xrplVerifierBatch, options);
        });
    addAmplifierOptions(verifierBatchCmd, { contractOptions: true });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
