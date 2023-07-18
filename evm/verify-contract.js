'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider, getContractAt } = ethers;
const { Command, Option } = require('commander');
const { verifyContract } = require('./utils');


async function verifyContracts(chain, options) {
    const { env, contractName } = options;
    const provider = getDefaultProvider(chain.rpc);
    const wallet = new Wallet('f'.repeat(64), provider);

    switch (contractName) {
        case 'Create3Deployer': {
            const Create3Deployer = require('@axelar-network/axelar-gmp-sdk-solidity/dist/Create3Deployer.json');

            const contractFactory = await getContractAt(Create3Deployer.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.Create3Deployer.address);

            console.log(`Verifying ${contractName} on ${chain.name} at address ${contract.address}...`);

            await verifyContract(env, chain.name, contract.address, []);
            break;
        }
    }
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

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];

        await verifyContracts(chain, options);
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
    program.addOption(new Option('-c, --contractName <contractName>', 'contract name'));
    program.addOption(new Option('-a, --address <address>', 'contract address'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
