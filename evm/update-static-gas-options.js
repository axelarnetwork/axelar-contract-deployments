'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { parseUnits },
} = ethers;
const readlineSync = require('readline-sync');

const { printInfo, mainProcessor } = require('./utils');

const defaultGasLimit = 3e6;
const gasPriceMultiplier = 5;

async function processCommand(config, chain, options) {
    const { rpc, yes } = options;
    const provider = rpc ? getDefaultProvider(rpc) : getDefaultProvider(chain.rpc);

    if (!yes) {
        const anwser = readlineSync.question(
            `Proceed with the static gasOption update on network ${chalk.green(chain.name)} ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    const gasPrice = parseUnits((await provider.getGasPrice()).toString(), 'wei') * gasPriceMultiplier;

    if (!(chain.staticGasOptions && chain.staticGasOptions.gasLimit !== undefined)) {
        chain.staticGasOptions = { gasLimit: defaultGasLimit };
    }

    chain.staticGasOptions.gasPrice = gasPrice;
    printInfo(`staticGasOptions updated succesfully and stored in config file`);
}

async function main(options) {
    await mainProcessor(options, processCommand, true);
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
program.addOption(new Option('-r, --rpc <rpc>', 'The rpc url for creating a provider to fetch gasOptions'));
program.addOption(new Option('-y, --yes', 'skip prompts'));

program.action((options) => {
    main(options);
});

program.parse();
