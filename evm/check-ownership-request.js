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
const { printError, mainProcessor, printInfo } = require('./utils');

const ERC20 = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/ERC20.sol/ERC20.json');
const IInterchainToken = require('@axelar-network/interchain-token-service/interfaces/IInterchainToken.json');

async function processCommand(_, chain, options) {
    const { rpc } = options;
    let { address, addresses } = options;

    try {
        addresses = JSON.parse(addresses);
    } catch (error) {
        printError(`Error while parsing addresses: `, error.message);
    }

    const provider = getDefaultProvider(rpc || chain.rpc);
    const erc20 = new Contract(address, ERC20.abi, provider);

    try {
        await checkContract(address, addresses, erc20);
        await checkAxelarDeployedToken(address, chain.name.toLowerCase(), provider);
        printInfo(`Contract verification checks passed.`);
    } catch (error) {
        printError(`Error while checking contract address`, error.message);
    }
}

async function checkContract(address, addressesArray, erc20) {
    const name = await erc20.name();
    const symbol = await erc20.symbol();
    const regex = /axl/i;

    if (addressesArray.some((addr) => addr.toLowerCase() === address.toLowerCase())) {
        throw new Error(`Contract address ${address} matches one of the specified addresses`);
    }

    if (regex.test(name) || regex.test(symbol)) {
        throw new Error('Contract name or symbol includes "axl"');
    }
}

async function checkAxelarDeployedToken(address, chain, provider) {
    let isAxelarDeployed = false;
    const apiUrl = `https://lcd-axelar.imperator.co/axelar/evm/v1beta1/token_info/${chain}?address=${address}`;

    try {
        const response = await axios.get(apiUrl);
        const data = response.data;
        isAxelarDeployed = data.confirmed === true && data.is_external === true;
    } catch (error) {}

    if (!isAxelarDeployed) {
        const interchainToken = new Contract(address, IInterchainToken.abi, provider);

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
        .description('Before signing the message check that the contract address provided is not for AXL or any of our important contract');

    addBaseOptions(program, { ignorePrivateKey: true, address: true });

    program.addOption(new Option('-r, --rpc <rpc>', 'The rpc url for creating a provider to fetch gasOptions'));
    program.addOption(
        new Option('--addresses <addresses>', 'The Array of addresses for which the address to check against').env('ADDRESSES'),
    );

    program.action((options) => {
        main(options);
    });

    program.parse();
}
