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
    printWalletInfo,
    mainProcessor,
    prompt,
    getGasOptions,
    dateToEta,
    createGMPProposalJSON,
    getGovernanceContract,
    getScheduleProposalType,
    writeJSON,
} = require('./utils');
const { addBaseOptions, addGovernanceOptions } = require('./cli-utils');
const { encodeGovernanceProposal, ProposalType, submitProposalToAxelar } = require('./governance');

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

    const buildGovernanceProposal = async (calldata) => {
        const { governanceContract, governanceAddress } = getGovernanceContract(chain, options);
        printInfo('Governance contract', governanceContract);
        const eta = dateToEta(options.activationTime || '0');
        const nativeValue = '0';
        const proposalType = getScheduleProposalType(options, ProposalType, action);
        const gmpPayload = encodeGovernanceProposal(proposalType, ownershipAddress, calldata, nativeValue, eta);

        printInfo('Governance target', ownershipAddress);
        printInfo('Governance calldata', calldata);

        return createGMPProposalJSON(chain, governanceAddress, gmpPayload);
    };

    if (options.governance && (action === 'owner' || action === 'pendingOwner')) {
        printInfo('Read-only ownership action; no governance proposal generated.');
        return null;
    }

    if (options.governance) {
        switch (action) {
            case 'transferOwnership': {
                if (!isAddress(newOwner) || newOwner === AddressZero) {
                    throw new Error(`Invalid new owner address: ${newOwner}`);
                }
                const { data: calldata } = await ownershipContract.populateTransaction.transferOwnership(newOwner, gasOptions);
                return buildGovernanceProposal(calldata);
            }
            case 'proposeOwnership': {
                if (!isAddress(newOwner) || newOwner === AddressZero) {
                    throw new Error(`Invalid new owner address: ${newOwner}`);
                }
                const { data: calldata } = await ownershipContract.populateTransaction.proposeOwnership(newOwner, gasOptions);
                return buildGovernanceProposal(calldata);
            }
            case 'acceptOwnership': {
                const { data: calldata } = await ownershipContract.populateTransaction.acceptOwnership(gasOptions);
                return buildGovernanceProposal(calldata);
            }
            default: {
                throw new Error(`Unknown ownership action ${action}`);
            }
        }
    }

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
    if (!options.governance) {
        await mainProcessor(options, processCommand);
        return;
    }

    const proposals = [];

    await mainProcessor(options, (axelar, chain, chains, opts) =>
        processCommand(axelar, chain, chains, opts).then((proposal) => {
            if (proposal) {
                proposals.push(proposal);
            }
        }),
    );

    if (proposals.length > 0) {
        const proposal = {
            title: 'Ownership Governance Proposal',
            description: 'Ownership Governance Proposal',
            contract_calls: proposals,
        };

        const proposalJSON = JSON.stringify(proposal, null, 2);

        printInfo('Proposal', proposalJSON);

        if (options.generateOnly) {
            writeJSON(proposal, options.generateOnly);
            printInfo('Proposal written to file', options.generateOnly);
        } else {
            if (!prompt('Proceed with submitting this proposal to Axelar?', options.yes)) {
                await submitProposalToAxelar(proposal, options);
            }
        }
    }
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
