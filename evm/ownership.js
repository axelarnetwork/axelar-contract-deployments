'use strict';

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
    constants: { AddressZero },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, printWalletInfo, loadConfig, saveConfig, prompt, getGasOptions } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

const IOwnable = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/interfaces/IOwnable.sol/IOwnable.json');

async function processCommand(options, chain) {
    const { contractName, address, action, privateKey, newOwner, yes } = options;

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

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

    printInfo('Contract name', contractName);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    const ownershipContract = new Contract(ownershipAddress, IOwnable.abi, wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('Ownership Action', action);

    if (prompt(`Proceed with ${action} on ${chain.name}?`, yes)) {
        return;
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
                throw new Error(`Caller ${wallet.address} is not the contract owner but ${owner} is.`);
            }

            if (!isAddress(newOwner) || newOwner === AddressZero) {
                throw new Error(`Invalid new owner address: ${newOwner}`);
            }

            try {
                await ownershipContract.transferOwnership(newOwner, gasOptions).then((tx) => tx.wait());
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
                await ownershipContract.proposeOwnership(newOwner, gasOptions).then((tx) => tx.wait());
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
                await ownershipContract.acceptOwnership(gasOptions).then((tx) => tx.wait());
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

if (require.main === module) {
    const program = new Command();

    program.name('ownership').description('script to manage contract ownership');

    addBaseOptions(program, { address: true });

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name'));
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

    program.action((options) => {
        main(options);
    });

    program.parse();
}
