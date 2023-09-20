'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const { getDefaultProvider } = ethers;
const readlineSync = require('readline-sync');

const { printError, printObj, loadConfig } = require('./utils');
const { getNonceFromProvider, getNonceFileData, updateNonceFileData } = require('./offline-sign-utils');

async function updateNonce(provider, env, chain, addresses) {
    try {
        const chainName = chain.name.toLowerCase();
        const nonceData = getNonceFileData();

        if (!nonceData[env]) {
            nonceData[env] = {};
        }

        if (!nonceData[env][chainName]) {
            nonceData[env][chainName] = {};
        }

        const chainNonceData = nonceData[env][chainName];

        if (addresses) {
            addresses = JSON.parse(addresses);

            for (const address of addresses) {
                const nonce = await getNonceFromProvider(provider, address);
                chainNonceData[address] = nonce;
            }
        } else {
            for (const [signerAddress] of Object.entries(chainNonceData)) {
                const nonce = await getNonceFromProvider(provider, signerAddress);
                chainNonceData[signerAddress] = nonce;
            }
        }

        nonceData[env][chainName] = chainNonceData;
        updateNonceFileData(nonceData);
    } catch (error) {
        printError(`Nonce updation failed with error: ${error.message}`);
        printObj(error);
    }
}

async function main(options) {
    const { env, chainNames, rpcUrl, addresses, yes } = options;
    const config = loadConfig(env);
    const chains = chainNames.split(',').map((str) => str.trim());

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];
        const provider = rpcUrl ? getDefaultProvider(rpcUrl) : getDefaultProvider(chain.rpc);
        const network = await provider.getNetwork();

        if (!yes) {
            const anwser = readlineSync.question(
                `Proceed with the nonces update on network ${chalk.green(network.name)} with chainId ${chalk.green(
                    network.chainId,
                )} ${chalk.green('(y/n)')} `,
            );
            if (anwser !== 'y') return;
        }

        await updateNonce(provider, env, chain, addresses);
    }
}

const program = new Command();

program.name('Update-Nonces').description('Offline sign all the unsigned transactions in the file');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(new Option('-r, --rpcUrl <rpcUrl>', 'The rpc url for creating a provider to fetch gasOptions'));
program.addOption(new Option('-a --addresses <addresses>', 'The Array of addresses for which the nonces to update').env('ADDRESSES'));
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});
program.parse();
