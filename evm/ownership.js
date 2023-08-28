'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
    constants: { AddressZero },
    ContractFactory,
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, printWalletInfo, loadConfig, saveConfig } = require('./utils');

async function processCommand(options, chain) {
    const { artifactPath, contractName, ownershipAction, privateKey, newOwner } = options;

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

    if (contractConfig && !contractConfig.address) {
        throw new Error(`Contract ${contractName} is not deployed on ${chain}`);
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Contract name', contractName);

    const contractPath = artifactPath + contractName + '.sol/' + contractName + '.json';
    printInfo('Contract path', contractPath);

    const contractJson = require(contractPath);
    const ownershipFactory = new ContractFactory(contractJson.abi, contractJson.bytecode, wallet);
    const ownershipContract = ownershipFactory.attach(contractConfig.address);

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    printInfo('Ownership Action', ownershipAction);

    switch (ownershipAction) {
        case 'getOwner': {
            const owner = await ownershipContract.owner();
            console.log(`Contract owner: ${owner}`);

            break;
        }

        case 'getPendingOwner': {
            const pendingOwner = await ownershipContract.pendingOwner();

            if (pendingOwner === AddressZero) {
                console.log('There is no pending owner.');
            } else {
                console.log(`Pending owner: ${pendingOwner}`);
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
            } else {
                console.log(`New contract owner: ${owner}`);
            }

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
            } else {
                console.log(`New pending owner: ${pendingOwner}`);
            }

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
            } else {
                console.log(`New contract owner: ${newOwner}`);
            }

            break;
        }

        default: {
            throw new Error(`Unknown ownership action ${ownershipAction}`);
        }
    }
}

async function main(options) {
    const config = loadConfig(options.env);

    const chain = options.chain;

    if (config.chains[chain.toLowerCase()] === undefined) {
        throw new Error(`Chain ${chain} is not defined in the info file`);
    }

    await processCommand(options, config.chains[chain.toLowerCase()], config);
    saveConfig(config, options.env);
}

const program = new Command();

program.name('ownership-script').description('script to manage contract ownership');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);

program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true));
program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
program.addOption(new Option('-n, --chain <chain>', 'chain name').makeOptionMandatory(true));
program.addOption(
    new Option('-o, --ownershipAction <ownershipAction>', 'ownership action').choices([
        'getOwner',
        'getPendingOwner',
        'transferOwnership',
        'proposeOwnership',
        'acceptOwnership',
    ]),
);
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-d, --newOwner <newOwner>', 'new owner address').makeOptionMandatory(false));

program.action((options) => {
    main(options);
});

program.parse();
