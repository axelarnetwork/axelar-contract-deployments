'use strict';

const { ethers } = require('hardhat');
const {
    providers: { JsonRpcProvider },
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, mainProcessor, sleep } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

function updateFinality(finality, chain, update) {
    if (update) {
        chain.finality = finality;
    }
}

async function processCommand(_config, chain, options) {
    let { rpc, confirmations, attempts, blockTime } = options;

    if (!rpc) {
        rpc = chain.rpc;
    }

    if (!attempts) {
        attempts = 200;
    }

    let max = 0;
    let min = Number.MAX_SAFE_INTEGER;
    let sum = 0;
    const DELAY = 10000;

    const provider = new JsonRpcProvider(rpc);

    try {
        await provider.getBlock('finalized');
    } catch {
        printInfo('Finalized tag not supported by rpc for chain', chain.name);

        if (confirmations) {
            confirmations = parseInt(confirmations);
            updateFinality(confirmations, chain, options.update);
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

        await sleep(DELAY);
    }

    const avg = sum / attempts;

    printInfo('Max difference', max);
    printInfo('Avg difference', avg);
    printInfo('Min difference', min);

    printInfo('Max wait time', max * blockTime);
    printInfo('Avg wait time', avg * blockTime);
    printInfo('Min wait time', min * blockTime);

    updateFinality('finalized', chain, options.update);
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
    program.addOption(new Option('--confirmations <Wait Time>', 'default wait time to add if finalized tag is not supported'));
    program.addOption(
        new Option('--attempts <Attempts>', 'number of attempts to calculate difference in block number to reach conclusion'),
    );
    program.addOption(new Option('--blockTime <Block Time>', 'difference in timestamp of 2 consecute blocks in seconds'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
