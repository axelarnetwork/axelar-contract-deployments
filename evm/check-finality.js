'use strict';

const { ethers } = require('hardhat');
const {
    providers: { JsonRpcProvider },
} = ethers;
const { Command } = require('commander');
const { printInfo, mainProcessor } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

async function processCommand(_config, chain, options) {
    const txHash = options.txHash;
    const rpc = chain.rpc;
    const provider = new JsonRpcProvider(rpc);
    const tx = await provider.getTransaction(txHash);
    const txBlock = tx.blockNumber;

    while (true) {
        const finalizedBlock = await provider.getBlock('finalized');

        if (finalizedBlock.number >= txBlock) {
            console.log('latest finalized block', finalizedBlock.number);
            printInfo('block for tx', txBlock);
            console.log('timestamp for finalized block', Date.now());
            break;
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('check-finality').description('Script to check when is finality achieved for a tx');
    addBaseOptions(program);
    program.addArgument('txHash', 'tx hash to check for finality');
    program.action((options) => {
        main(options);
    });

    program.parse();
}
