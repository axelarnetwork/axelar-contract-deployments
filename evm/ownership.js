'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
    constants: { AddressZero },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, printWalletInfo, loadConfig, saveConfig } = require('./utils');
const readlineSync = require('readline-sync');
const chalk = require('chalk');

const IOwnable = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/interfaces/IOwnable.sol/IOwnable.json');

async function processCommand(options, chain) {
    const { contractName, address, action, privateKey, newOwner, yes } = options;

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

    let ownershipAddress;

    if (isAddress(address)) {
        ownershipAddress = address;
    } else {
        if (!contractConfig?.address) {
            throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
        }

        ownershipAddress = contractConfig.address;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    printInfo('Chain', chain.name);
    printInfo('Contract name', contractName);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    const ownershipContract = new Contract(ownershipAddress, IOwnable.abi, wallet);

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    printInfo(`Gas override for ${chain.name}`, JSON.stringify(gasOptions));

    printInfo('Ownership Action', action);

    if (!yes) {
        const anwser = readlineSync.question(`Proceed with action on ${chain.name}? ${chalk.green('(y/n)')} `);
        if (anwser !== 'y') return;
    }

    switch (action) {
        case 'owner': {
            const owner = await ownershipContract.owner();
            printInfo(`Contract owner: ${owner}`);

            break;
        }

        case 'pendingOwner': {
            const pendingOwner = await ownershipContract.pendingOwner();

            if (pendingOwner === AddressZero) {
                printInfo('There is no pending owner.');
            } else {
                printInfo(`Pending owner: ${pendingOwner}`);
            }

            break;
        }

        case 'transferOwnership': {
            let owner = await ownershipContract.owner();

            if (owner.toLowerCase() !== wallet.address.toLowerCase()) {
                throw new Error(`Caller ${wallet.address} is not the contract owner.`);
            }

            if (!isAddress(newOwner) || newOwner === AddressZero) {
                throw new Error(`Invalid new owner address: ${newOwner}`);
            }

            try {
                await ownershipContract.transferOwnership(newOwner).then((tx) => tx.wait());
            } catch (error) {
                throw new Error(error);
            }

            owner = await ownershipContract.owner();

            if (owner.toLowerCase() !== newOwner.toLowerCase()) {
                throw new Error('Ownership transfer failed.');
            }

            printInfo(`New contract owner: ${owner}`);

            contractConfig.owner = owner;

            break;
        }

        case 'proposeOwnership': {
            const owner = await ownershipContract.owner();

            if (owner.toLowerCase() !== wallet.address.toLowerCase()) {
                throw new Error(`Caller ${wallet.address} is not the contract owner.`);
            }

            if (!isAddress(newOwner) || newOwner === AddressZero) {
                throw new Error(`Invalid new owner address: ${newOwner}`);
            }

            try {
                await ownershipContract.proposeOwnership(newOwner).then((tx) => tx.wait());
            } catch (error) {
                throw new Error(error);
            }

            const pendingOwner = await ownershipContract.pendingOwner();

            if (pendingOwner.toLowerCase() !== newOwner.toLowerCase()) {
                throw new Error('Propose ownership failed.');
            }

            printInfo(`New pending owner: ${pendingOwner}`);

            break;
        }

        case 'acceptOwnership': {
            const pendingOwner = await ownershipContract.pendingOwner();

            if (pendingOwner === AddressZero) {
                throw new Error('This is no pending owner.');
            }

            if (pendingOwner.toLowerCase() !== wallet.address.toLowerCase()) {
                throw new Error(`Caller ${wallet.address} is not the pending owner.`);
            }

            try {
                await ownershipContract.acceptOwnership().then((tx) => tx.wait());
            } catch (error) {
                throw new Error(error);
            }

            const newOwner = await ownershipContract.owner();

            if (newOwner.toLowerCase() !== pendingOwner.toLowerCase()) {
                throw new Error('Accept ownership failed.');
            }

            printInfo(`New contract owner: ${newOwner}`);

            contractConfig.owner = newOwner;

            break;
        }

        default: {
            throw new Error(`Unknown ownership action ${action}`);
        }
    }
}

async function main(options) {
    const config = loadConfig(options.env);

    let chains = options.chainNames.split(',').map((str) => str.trim());

    if (options.chainNames === 'all') {
        chains = Object.keys(config.chains);
    }

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chain} is not defined in the info file`);
        }
    }

    for (const chain of chains) {
        await processCommand(options, config.chains[chain.toLowerCase()]);
        saveConfig(config, options.env);
    }
}

const program = new Command();

program.name('ownership').description('script to manage contract ownership');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);

program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(new Option('--address <address>', 'override address').makeOptionMandatory(false));
program.addOption(
    new Option('--action <action>', 'ownership action').choices([
        'owner',
        'pendingOwner',
        'transferOwnership',
        'proposeOwnership',
        'acceptOwnership',
    ]),
);
program.addOption(new Option('--newOwner <newOwner>', 'new owner address').makeOptionMandatory(false));
program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

program.action((options) => {
    main(options);
});

program.parse();
