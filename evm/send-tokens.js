'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider } = ethers;
const { Command, Option } = require('commander');
const chalk = require('chalk');

async function sendTokens(chain, options) {
    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet(options.privateKey, provider);

    const tx = await wallet.sendTransaction({
        to: options.recipient,
        value: ethers.utils.parseEther(options.amount),
    });

    console.log(`Transaction hash: ${chalk.green(tx.hash)}`);

    await tx.wait();
}

async function main(options) {
    const config = require(`${__dirname}/../info/${options.env === 'local' ? 'testnet' : options.env}.json`);

    const chains = options.chainNames.split(',').map((str) => str.trim());

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];

        await sendTokens(chain, options);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('send-tokens').description('Send native tokens to an address.');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-r, --recipient <recipient>', 'recipient of tokens').makeOptionMandatory(true));
    program.addOption(new Option('-a, --amount <amount>', 'amount to transfer (in terms of ETH)').makeOptionMandatory(true));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokens };
}
