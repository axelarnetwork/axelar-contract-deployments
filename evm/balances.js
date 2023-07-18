'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider } = ethers;
const { Command, Option } = require('commander');
const { printInfo } = require('./utils');


async function balances(chain, options) {
    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet(options.privateKey, provider);

    const balance = await wallet.getBalance();

    printInfo(chain.name, `${balance / 1e18} ${chain.tokenSymbol}`);
}

async function main(options) {
    const config = require(`${__dirname}/../info/${options.env === 'local' ? 'testnet' : options.env}.json`);

    let chains = options.chainNames.split(',').map((str) => str.trim());

    if (options.chainNames === 'all') {
        chains = Object.keys(config.chains);
    }

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    const wallet = new Wallet(options.privateKey);
    printInfo('Wallet address', wallet.address);

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];

        await balances(chain, options);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('balances').description('Display balance of the wallet on specified chains.');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
