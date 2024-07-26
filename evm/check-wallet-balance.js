'use strict';

const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const { getDefaultProvider, BigNumber } = ethers;

const { printError, mainProcessor } = require('./utils');
const { addBaseOptions } = require('../common');
const { getNonceFileData } = require('./sign-utils');

async function processCommand(_, chain, options) {
    const { rpc } = options;
    let { addresses } = options;

    const chainName = chain.name.toLowerCase();
    const provider = getDefaultProvider(rpc || chain.rpc);
    const staticGasOptions = chain.staticGasOptions;

    if (!staticGasOptions) {
        printError('Could not find staticGasOptions for chain', chain.name);
        return;
    }

    const gasLimit = BigNumber.from(chain.staticGasOptions.gasLimit);
    const gasPrice = BigNumber.from(chain.eip1559 ? staticGasOptions.maxFeePerGas : staticGasOptions.gasPrice);
    const minRequiredBalance = gasLimit * gasPrice * 1.5;
    printError(`${chain.name} minimum required Balance`, `${minRequiredBalance / 1e18}`);

    const nonceData = getNonceFileData();
    const nonces = nonceData[options.env][chainName];

    if (addresses) {
        addresses = JSON.parse(addresses);
    } else {
        addresses = Object.keys(nonces);
    }

    for (const address of addresses) {
        const balance = await provider.getBalance(address);

        if (balance < minRequiredBalance) {
            printError(`${chain.name} Wallet Balance for ${address} is`, `${balance / 1e18}`);
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('check-wallet-balance').description('Before offline signing checks if each signer has minimum required wallet balance');

    addBaseOptions(program, { ignorePrivateKey: true });

    program.addOption(new Option('-r, --rpc <rpc>', 'The rpc url for creating a provider to fetch gasOptions'));
    program.addOption(new Option('--addresses <addresses>', 'The Array of addresses for which the balance to check').env('ADDRESSES'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
