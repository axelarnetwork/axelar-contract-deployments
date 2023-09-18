'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider
} = ethers;
const readlineSync = require('readline-sync');

const { printError, printInfo, printObj, saveConfig } = require('./utils');

async function updateStaticGasOptions(chain, options, filePath) {
    const{rpcUrl, yes} = options;
    const provider = getDefaultProvider(rpcUrl);
    const network = await provider.getNetwork();

    if (!yes) {
        const anwser = readlineSync.question(
            `Proceed with the static gasOption update on network ${chalk.green(network.name)} with chainId ${chalk.green(network.chainId)} ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }
    try {
        const gasPrice = await provider.getGasPrice() * 5;
        const block = await provider.getBlock('latest');
        const gasLimit = block.gasLimit.toNumber() / 500;
        const staticGasOptions = {};
        staticGasOptions.gasLimit = gasLimit;
        staticGasOptions.gasPrice = gasPrice;
        chain.staticGasOptions = staticGasOptions;
        console.log("chain");
        printObj(chain);

        printInfo(`GasOptions updated succesfully and stored in config file ${filePath}`);
    } catch(error) {
        printError(`GasOptions updation failed with error: ${error.message}`);
        printObj(error);
    }
    return chain;
}


async function main(options) {
    const { env, chainNames } = options;

    const filePath = `${__dirname}/../axelar-chains-config/info/${env === 'local' ? 'testnet' : env}.json`
    const config = require(filePath);

    const chains = chainNames.split(',').map((str) => str.trim());

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];

        config.chains[chainName.toLowerCase()] = await updateStaticGasOptions(chain, options, filePath);
        console.log("Config");
        printObj(config);
    }
    saveConfig(config, env);
}

const program = new Command();

program.name('Update-GasOptions').description('Update gasOptions to be used in offline signing');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(
    new Option('-r, --rpcUrl <rpcUrl>', 'The rpc url for creating a provider to fetch gasOptions').makeOptionMandatory(true),
);
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});

program.parse();