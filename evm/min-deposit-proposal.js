'use strict';

require('dotenv').config();

const { Command, Option } = require('commander');
const { mainProcessor, printInfo, isValidNumber, isValidAddress } = require('./utils');

const values = [];

async function processCommand(_, chain, options) {
    const { address, deposit } = options;

    printInfo('Chain', chain.name);

    const contracts = chain.contracts;
    const contractConfig = contracts.InterchainGovernance;

    let governanceAddress;

    if (isValidAddress(address)) {
        governanceAddress = address;
    } else {
        if (!contractConfig?.address) {
            throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
        }

        governanceAddress = contractConfig.address;
    }

    if (!isValidNumber(deposit)) {
        throw new Error('Invalid deposit amount');
    }

    values.push({
        chain: chain.id.toLowerCase(),
        'contract_address': governanceAddress,
        'min_deposits': [
            {
                denom: 'uaxl',
                amount: `${parseInt(deposit) * 1e6}`,
            }
        ]
    });
}

async function main(options) {
    await mainProcessor(options, processCommand);

    const paramChange = {
        title: 'Update min deposit for governance proposals',
        description: `This proposal sets a minimum deposit of ${options.deposit} AXL for any governance proposals for the Axelar gateway contracts.`,
        deposit: '2000000000uaxl',
        changes: [{
            subspace: 'axelarnet',
            key: 'callContractsProposalMinDeposits',
            value: values,
        }],
    }

    printInfo('Proposal', JSON.stringify(paramChange, null, 2));
}

if (require.main === module) {
    const program = new Command();

    program.name('balances').description('Display balance of the wallet on specified chains.');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true).env('CHAINS'));
    program.addOption(new Option('-a, --address <address>', 'override governance address'));
    program.addOption(new Option('--deposit <deposit>', 'min deposit for governance proposals, in terms of AXL'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
