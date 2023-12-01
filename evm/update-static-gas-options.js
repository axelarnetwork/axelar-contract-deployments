'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { parseUnits },
    BigNumber,
} = ethers;

const { printInfo, mainProcessor, prompt } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

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

const minGasLimits = {
    mainnet: {
        filecoin: 3e8,
        arbitrum: 20e8,
    },
    testnet: {
        filecoin: 3e8,
        arbitrum: 20e8,
    },
};

async function getBaseFee(provider) {
    const block = await provider.getBlock('latest');
    return block.baseFeePerGas;
}

async function processCommand(_, chain, options) {
    const { env, rpc, yes } = options;
    const provider = getDefaultProvider(rpc || chain.rpc);

    if (prompt(`Proceed with the static gasOption update on ${chalk.green(chain.name)}`, yes)) {
        return;
    }

    let gasPriceWei = await provider.getGasPrice();

    if (chain.eip1559) {
        const baseFee = await getBaseFee(provider);
        const maxPriorityFeePerGas = await provider.send('eth_maxPriorityFeePerGas', []);
        gasPriceWei = BigNumber.from(baseFee).add(BigNumber.from(maxPriorityFeePerGas));
    }

    printInfo(`${chain.name} gas price`, `${gasPriceWei / 1e9} gwei`);

    let gasPrice = parseUnits(gasPriceWei.toString(), 'wei') * gasPriceMultiplier;

    const minGasLimit = (minGasLimits[env] || {})[chain.name.toLowerCase()] || defaultGasLimit;

    if (!(chain.staticGasOptions && chain.staticGasOptions.gasLimit !== undefined)) {
        chain.staticGasOptions = { gasLimit: minGasLimit };
    }

    const minGasPrice = ((minGasPrices[env] || {})[chain.name.toLowerCase()] || 0) * 1e9;
    gasPrice = gasPrice < minGasPrice ? minGasPrice : gasPrice;

    if (chain.eip1559) {
        chain.staticGasOptions.maxFeePerGas = gasPrice;
    } else {
        chain.staticGasOptions.gasPrice = gasPrice;
    }

    printInfo(`${chain.name} static gas price set to`, `${gasPrice / 1e9} gwei`);

    printInfo(`staticGasOptions updated succesfully and stored in config file`);
}

async function main(options) {
    await mainProcessor(options, processCommand, true);
}

if (require.main === module) {
    const program = new Command();

    program.name('update-static-gas-options').description('Update staticGasOptions');

    addBaseOptions(program, { ignorePrivateKey: true });

    program.addOption(new Option('-r, --rpc <rpc>', 'The rpc url for creating a provider to fetch gasOptions'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
