'use strict';

const { ethers } = require('hardhat');
const {
    providers: { JsonRpcProvider },
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, mainProcessor, isKeccak256Hash, sleep } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

async function processCommand(_config, chain, options) {
    const txHash = options.txHash;

    if (!isKeccak256Hash(txHash)) {
        throw new Error(`Invalid tx format: ${txHash}`);
    }

    const rpc = chain.rpc;
    const provider = new JsonRpcProvider(rpc);
    const tx = await provider.getTransaction(txHash);
    const txBlock = tx.blockNumber;

    while (true) {
        const finalizedBlock = await provider.getBlock('finalized');

        if (finalizedBlock.number >= txBlock) {
            printInfo('Latest finalized block', finalizedBlock.number);
            printInfo('Block for tx', txBlock);
            printInfo('Timestamp for finalized block', Date.now());
            break;
        }

        await sleep(1000);
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('check-finality').description('Script to check when is finality achieved for a tx');
    addBaseOptions(program);
    program.addOption(new Option('-t, --txHash <txHash>', 'tx hash to check for finality').makeOptionMandatory(true));
    program.action((options) => {
        main(options);
    });

    program.parse();
}
