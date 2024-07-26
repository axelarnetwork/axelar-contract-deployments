'use strict';

const { ethers } = require('hardhat');
const {
    providers: { JsonRpcProvider },
} = ethers;
const { Command, Option } = require('commander');
const { mainProcessor, sleep } = require('./utils');
const { printInfo, addBaseOptions } = require('../common');

function updateFinality(finality, chain, update) {
    if (update) {
        chain.finality = finality;
    }
}

function updateFinalityWaitTime(approxFinalityWaitTime, chain, update) {
    if (update) {
        chain.approxFinalityWaitTime = approxFinalityWaitTime;
    }
}

async function processCommand(_config, chain, options) {
    let { confirmations, attempts, blockTime, delay } = options;

    const rpc = options.rpc || chain.rpc;

    let max = 0;
    let min = Number.MAX_SAFE_INTEGER;
    let sum = 0;

    const provider = new JsonRpcProvider(rpc);

    try {
        await provider.getBlock('finalized');
    } catch {
        printInfo('Finalized tag not supported by rpc for chain', chain.name);

        if (confirmations) {
            confirmations = parseInt(confirmations);
            updateFinality(confirmations, chain, options.update);
            updateFinalityWaitTime(confirmations * blockTime, chain, options.update);
            printInfo('Wait time', confirmations * blockTime);
        }

        return;
    }

    for (let i = 0; i < attempts; i++) {
        const latestBlockPromise = provider.getBlock('latest'); // resolved promises afterwards to make difference more precise
        const finalizedBlockPromise = provider.getBlock('finalized');

        const latestBlock = await latestBlockPromise;
        const finalizedBlock = await finalizedBlockPromise;

        const difference = latestBlock.number - finalizedBlock.number;

        printInfo(`Difference in block number for ${i + 1} time`, difference);

        sum += difference;
        max = difference > max ? difference : max;
        min = difference < min ? difference : min;

        await sleep(delay);
    }

    const avg = sum / attempts;

    printInfo('Max difference', max);
    printInfo('Avg difference', avg);
    printInfo('Min difference', min);

    printInfo('Max wait time', max * blockTime);
    printInfo('Avg wait time', avg * blockTime);
    printInfo('Min wait time', min * blockTime);

    updateFinality('finalized', chain, options.update);
    updateFinalityWaitTime(max * blockTime, chain, options.update);
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('determine-finality-latency')
        .description(
            'Determine the latency between finalized and latest blocks based on finalized tag or block confirmations, and update chain config',
        );

    addBaseOptions(program);

    program.addOption(new Option('--rpc <rpc>', 'chain rpc'));
    program.addOption(new Option('--update', 'update finality setting in the chain config based on result'));
    program.addOption(new Option('--confirmations <confirmations>', 'default wait time to add if finalized tag is not supported'));
    program.addOption(
        new Option('--attempts <attempts>', 'number of attempts to calculate difference in block number to reach conclusion').default(200),
    );
    program.addOption(new Option('--blockTime <blockTime>', 'default block confirmations to wait for if finalized tag is not supported'));
    program.addOption(new Option('--delay <delay>', 'delay between calculating consecutive block finality differences').default(10));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
