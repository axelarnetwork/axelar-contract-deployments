'use strict';

const { ethers } = require('hardhat');
const {
    providers: { JsonRpcProvider },
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, mainProcessor } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

async function processCommand(config, chain, options) {
    let { rpc, waitTime, update } = options;

    if (!rpc) {
        rpc = chain.rpc;
    }

    let max = 0;
    let sum = 0;
    const DELAY = 10000;

    const provider = new JsonRpcProvider(rpc);

    try {
        await provider.getBlock('finalized');
    } catch {
        if (update && waitTime) {
            chain.finality = parseInt(waitTime);
        }

        printInfo('Finalized tag not supported by rpc');
        return;
    }

    for (let i = 0; i < 10; i++) {
        const latestBlockPromise = provider.getBlock('latest');
        const finalizedBlockPromise = provider.getBlock('finalized');

        const latestBlock = await latestBlockPromise;
        const finalizedBlock = await finalizedBlockPromise;

        const difference = latestBlock.number - finalizedBlock.number;

        printInfo(`Difference in block number for ${i + 1} time`, difference);

        sum += difference;
        max = difference > max ? difference : max;

        await new Promise((resolve) => setTimeout(resolve, DELAY));
    }

    printInfo('Max difference', max);
    printInfo('Avg difference', sum / 10);

    if (update) {
        chain.finality = 'finalized';
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('print difference b/w finalized block and latest block number')
        .description('Script to print difference in block numbers and update config for if the finalized tag is supported on that chain');

    addBaseOptions(program);

    program.addOption(new Option('--rpc <RPC>', 'rpc to use to calculdate difference'));
    program.addOption(new Option('--update', 'update configurations based on result'));
    program.addOption(new Option('--waitTime <Wait Time>', 'default wait time to add if finalized tag is not supported'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
