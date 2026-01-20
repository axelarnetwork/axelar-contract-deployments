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
const {
    printInfo,
    printWarn,
    prompt,
    validateParameters,
    printWalletInfo,
    mainProcessor,
    getGasOptions,
    executeDirectlyOrSubmitProposal,
} = require('./utils');
const { addBaseOptions, addGovernanceOptions } = require('./cli-utils');

const IOwnable = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/interfaces/IOwnable.sol/IOwnable.json');

async function processCommand(_axelar, chain, _chains, options) {
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
    await printWalletInfo(wallet, options, chain);

    const ownershipContract = new Contract(ownershipAddress, IOwnable.abi, wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('Ownership Action', action);

    const needsConfirmation = !options.governance && ['transferOwnership', 'proposeOwnership', 'acceptOwnership'].includes(action);
    if (needsConfirmation && prompt(`Proceed with ${action} on ${chain.name}?`, yes)) {
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
            validateParameters({
                isValidAddress: { newOwner },
            });

            if (!options.governance) {
                const currentOwner = await ownershipContract.owner();
                if (currentOwner.toLowerCase() !== wallet.address.toLowerCase()) {
                    throw new Error(`Caller ${wallet.address} is not the contract owner but ${currentOwner} is.`);
                }
            }

            await executeDirectlyOrSubmitProposal(chain, ownershipContract, 'transferOwnership', [newOwner], options, '0', [
                'OwnershipTransferred',
            ]);

            if (options.governance) {
                printInfo('Governance proposal submitted');
                break;
            }

            const owner = await ownershipContract.owner();

            if (owner.toLowerCase() !== newOwner.toLowerCase()) {
                throw new Error('Ownership transfer failed.');
            }

            printInfo(`New contract owner: ${owner}`);

            contractConfig.owner = owner;

            break;
        }

        case 'proposeOwnership': {
            validateParameters({
                isValidAddress: { newOwner },
            });

            if (!options.governance) {
                const currentOwner = await ownershipContract.owner();
                if (currentOwner.toLowerCase() !== wallet.address.toLowerCase()) {
                    throw new Error(`Caller ${wallet.address} is not the contract owner.`);
                }
            }

            await executeDirectlyOrSubmitProposal(chain, ownershipContract, 'proposeOwnership', [newOwner], options, '0', [
                'OwnershipTransferStarted',
            ]);

            if (options.governance) {
                printInfo('Governance proposal submitted');
                break;
            }

            const pendingOwner = await ownershipContract.pendingOwner();

            if (pendingOwner.toLowerCase() !== newOwner.toLowerCase()) {
                throw new Error('Propose ownership failed.');
            }

            printInfo(`New pending owner: ${pendingOwner}`);

            break;
        }

        case 'acceptOwnership': {
            if (newOwner) {
                printWarn('--newOwner is ignored for acceptOwnership action.');
            }

            let pendingOwner;

            if (!options.governance) {
                pendingOwner = await ownershipContract.pendingOwner();
                if (pendingOwner === AddressZero) {
                    throw new Error('There is no pending owner.');
                }

                if (pendingOwner.toLowerCase() !== wallet.address.toLowerCase()) {
                    throw new Error(`Caller ${wallet.address} is not the pending owner.`);
                }
            }

            await executeDirectlyOrSubmitProposal(chain, ownershipContract, 'acceptOwnership', [], options, '0', ['OwnershipTransferred']);

            if (options.governance) {
                printInfo('Governance proposal submitted');
                break;
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
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('ownership').description('script to manage contract ownership');

    addBaseOptions(program, { address: true });
    addGovernanceOptions(program);

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
