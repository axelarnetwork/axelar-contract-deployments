'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { parseUnits },
} = ethers;
const readlineSync = require('readline-sync');

const { printError, printInfo, printObj, saveConfig, loadConfig } = require('./utils');

async function updateStaticGasOptions(chain, options, filePath) {
    const { rpcUrl, yes } = options;
    const provider = rpcUrl ? getDefaultProvider(rpcUrl) : getDefaultProvider(chain.rpc);
    const network = await provider.getNetwork();

    if (!yes) {
        const anwser = readlineSync.question(
            `Proceed with the static gasOption update on network ${chalk.green(network.name)} with chainId ${chalk.green(
                network.chainId,
            )} ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    try {
        const gasPrice = parseUnits((await provider.getGasPrice()).toString(), 'gwei') * 5;

        if (!(chain.staticGasOptions && chain.staticGasOptions.gasLimit !== undefined)) {
            chain.staticGasOptions = { gasLimit: 3e6 };
        }

        chain.staticGasOptions.gasPrice = gasPrice;
        printInfo(`GasOptions updated succesfully and stored in config file ${filePath}`);
    } catch (error) {
        printError(`GasOptions updation failed with error: ${error.message}`);
        printObj(error);
    }

    return chain;
}

async function main(options) {
    const { env, chainNames } = options;
    const filePath = `${__dirname}/../axelar-chains-config/info/${env}.json`;
    const config = loadConfig(env);
    const chains = chainNames.split(',').map((str) => str.trim());

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];
        config.chains[chainName.toLowerCase()] = await updateStaticGasOptions(chain, options, filePath);
    }

    saveConfig(config, env);
}

const program = new Command();

program.name('update-static-gas-options').description('Update staticGasOptions to be used when offline signing');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(new Option('-r, --rpcUrl <rpcUrl>', 'The rpc url for creating a provider to fetch gasOptions'));
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});

program.parse();
