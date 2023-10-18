'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { parseUnits },
} = ethers;

const { printInfo, mainProcessor, prompt } = require('./utils');

const defaultGasLimit = 3e6;
const gasPriceMultiplier = 5;

const minGasPrices = {
    mainnet: {
        ethereum: 150,
        moonbeam: 500,
        avalanche: 150,
        polygon: 350,
        fantom: 1000,
        binance: 30,
        arbitrum: 2,
        celo: 100,
        kava: 50,
        optimism: 10,
        filecoin: 1,
        base: 10,
        linea: 10,
        mantle: 25,
        scroll: 25,
    },
    testnet: {
        mantle: 1,
    },
};

async function processCommand(_, chain, options) {
    const { env, rpc, yes } = options;
    const provider = rpc ? getDefaultProvider(rpc) : getDefaultProvider(chain.rpc);

    if (prompt(`Proceed with the static gasOption update on ${chalk.green(chain.name)}`, yes)) {
        return;
    }

    const gasPriceWei = await provider.getGasPrice();
    printInfo(`${chain.name} gas price`, `${gasPriceWei / 1e9} gwei`);

    const gasPrice = parseUnits(gasPriceWei.toString(), 'wei') * gasPriceMultiplier;

    if (!(chain.staticGasOptions && chain.staticGasOptions.gasLimit !== undefined)) {
        chain.staticGasOptions = { gasLimit: defaultGasLimit };
    }

    const minGasPrice = ((minGasPrices[env] || {})[chain.name.toLowerCase()] || 0) * 1e9;
    chain.staticGasOptions.gasPrice = gasPrice < minGasPrice ? minGasPrice : gasPrice;

    printInfo(`${chain.name} static gas price set to`, `${chain.staticGasOptions.gasPrice / 1e9} gwei`);

    printInfo(`staticGasOptions updated succesfully and stored in config file`);
}

async function main(options) {
    await mainProcessor(options, processCommand, true);
}

const program = new Command();

program.name('update-static-gas-options').description('Update staticGasOptions');

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
