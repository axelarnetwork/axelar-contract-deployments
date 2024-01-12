'use strict';

const axios = require('axios');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    Contract,
    constants: { AddressZero },
} = ethers;

const { addBaseOptions } = require('./cli-utils');
const { printError, mainProcessor, printInfo, getContractJSON, isValidAddress } = require('./utils');

async function processCommand(_, chain, options) {
    const { rpc } = options;
    let { address, addresses } = options;

    if(!isValidAddress(address)) {
        throw new Error('Invalid address parameter.');
    }

    try {
        addresses = JSON.parse(addresses);
    } catch (error) {
        printError(`Error while parsing addresses: `, error.message);
    }

    for(const index in addresses) {
        if(!isValidAddress(addresses[index])) {
            throw new Error(`Invalid address at index ${index} in addresses parameter.`);
        }
    }

    const provider = getDefaultProvider(rpc || chain.rpc);
    const erc20ABI = getContractJSON('ERC20').abi;
    const erc20 = new Contract(address, erc20ABI, provider);

    try {
        await checkContract(address, addresses, erc20);
        await checkAxelarDeployedToken(address, chain.name.toLowerCase(), provider);
        printInfo(`Contract verification checks passed.`);
    } catch (error) {
        printError(`Error while checking contract address`, error.message);
    }
}

async function checkContract(address, addressesArray, erc20) {
    if (addressesArray.some((addr) => addr.toLowerCase() === address.toLowerCase())) {
        throw new Error(`Contract address ${address} matches one of the specified addresses`);
    }
}

async function checkAxelarDeployedToken(address, chain, provider) {
    let isAxelarDeployed = false;
    const apiUrl = `https://lcd-axelar.imperator.co/axelar/evm/v1beta1/token_info/${chain}?address=${address}`;

    try {
        const response = await axios.get(apiUrl);
        const data = response.data;
        isAxelarDeployed = data.confirmed === true && data.is_external === false;
    } catch (error) {}

    if (!isAxelarDeployed) {
        const interchainTokenABI = getContractJSON('InterchainToken').abi;
        const interchainToken = new Contract(address, interchainTokenABI, provider);

        try {
            const itsAddress = await interchainToken.interchainTokenService();
            isAxelarDeployed = itsAddress !== AddressZero;
        } catch (error) {
            isAxelarDeployed = false;
        }

        if (!isAxelarDeployed) {
            throw new Error('Contract is not deployed by our Gateway');
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('check-ownership-request')
        .description('Before signing the message check that the contract address provided is not for AXL or any of our other important contracts');

    addBaseOptions(program, { ignorePrivateKey: true, address: true });

    program.addOption(new Option('-r, --rpc <rpc>', 'The rpc url for creating a provider to fetch gasOptions'));
    program.addOption(
        new Option('--addresses <addresses>', 'The Array of addresses that the address will be checked against').env('ADDRESSES'),
    );

    program.action((options) => {
        main(options);
    });

    program.parse();
}
