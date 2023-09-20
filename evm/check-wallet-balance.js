'use strict';

const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const { getDefaultProvider, BigNumber } = ethers;

const { printError, printObj, loadConfig } = require('./utils');
const { getNonceFileData } = require('./offline-sign-utils');

async function checkBalance(provider, env, chain, staticGasOptions, addresses) {
    try {
        const minRequiredBalance = BigNumber.from(staticGasOptions.gasLimit * staticGasOptions.gasPrice);
        const chainName = chain.name.toLowerCase();
        const nonceData = getNonceFileData();
        const chainNonceData = nonceData[env][chainName];

        if (addresses) {
            addresses = JSON.parse(addresses);

            for (const address of addresses) {
                const balance = await provider.getBalance(address);

                if (balance < minRequiredBalance) {
                    printError('Minimum required Balance is', minRequiredBalance);
                    printError(`Wallet Balance for address ${address} is less than minimum required amount. Wallet Balance: `, balance);
                }
            }
        } else {
            for (const [address] of Object.entries(chainNonceData)) {
                const balance = await provider.getBalance(address);

                if (balance < minRequiredBalance) {
                    printError('Minimum required Balance is', minRequiredBalance);
                    printError(`Wallet Balance for address ${address} is less than minimum required amount. Wallet Balance: `, balance);
                }
            }
        }
    } catch (error) {
        printError(`Checking wallet balance failed with error: ${error.message}`);
        printObj(error);
    }
}

async function main(options) {
    const { env, chainNames, rpcUrl, addresses } = options;
    const config = loadConfig(env);
    const chains = chainNames.split(',').map((str) => str.trim());

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];
        const staticGasOptions = chain.staticGasOptions;

        if (!staticGasOptions) {
            printError('Could not find staticGasOptions for chain ', chain.name.toLowerCase());
            continue;
        }

        const provider = rpcUrl ? getDefaultProvider(rpcUrl) : getDefaultProvider(chain.rpc);
        await checkBalance(provider, env, chain, staticGasOptions, addresses);
    }
}

const program = new Command();

program.name('check-wallet-balance').description('Before offline signing checks if each signer has minimum required wallet balance');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(new Option('-r, --rpcUrl <rpcUrl>', 'The rpc url for creating a provider to fetch gasOptions'));
program.addOption(new Option('-a --addresses <addresses>', 'The Array of addresses for which the balance to check').env('ADDRESSES'));

program.action((options) => {
    main(options);
});

program.parse();
