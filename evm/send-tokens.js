'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider } = ethers;
const { Command, Option } = require('commander');
const chalk = require('chalk');
const { printInfo } = require('./utils');
const readlineSync = require('readline-sync');

async function sendTokens(chain, options) {
    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet(options.privateKey, provider);

    printInfo('Wallet address', wallet.address);
    const balance = await wallet.provider.getBalance(wallet.address);
    const amount = ethers.utils.parseEther(options.amount);

    console.log(
        `Wallet has ${balance / 1e18} ${chalk.green(chain.tokenSymbol)} and nonce ${await wallet.provider.getTransactionCount(
            wallet.address,
        )} on ${chain.name}.`,
    );

    if (balance.lte(amount)) {
        throw new Error(`Wallet has insufficient funds.`);
    }

    const recipients = options.recipients.split(',').map((str) => str.trim());

    if (!options.yes) {
        const anwser = readlineSync.question(
            `Proceed with the transfer of ${chalk.green(options.amount)} ${chalk.green(chain.tokenSymbol)} to ${recipients} on ${
                chain.name
            }? ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    for (const recipient of recipients) {
        printInfo('Recipient', recipient);

        const tx = await wallet.sendTransaction({
            to: recipient,
            value: amount,
        });

        printInfo('Transaction hash', tx.hash);

        await tx.wait();
    }
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
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-r, --recipients <recipients>', 'comma-separated recipients of tokens').makeOptionMandatory(true));
    program.addOption(new Option('-a, --amount <amount>', 'amount to transfer (in terms of ETH)').makeOptionMandatory(true));
    program.addOption(new Option('-y, --yes', 'skip prompts'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokens };
}
