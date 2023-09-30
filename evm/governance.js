'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { defaultAbiCoder, keccak256, Interface },
    Contract,
    BigNumber,
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    printWalletInfo,
    isValidTimeFormat,
    dateToEta,
    etaToDate,
    getCurrentTimeInSeconds,
    wasEventEmitted,
    printWarn,
    printError,
    getBytecodeHash,
    isValidAddress,
    mainProcessor,
    isValidDecimal,
    prompt,
} = require('./utils');
const { getWallet } = require('./sign-utils.js');
const IGovernance = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarServiceGovernance.json');
const IGateway = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGateway.json');

async function processCommand(_, chain, options) {
    const { contractName, address, action, nativeValue, date, privateKey, yes } = options;

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName];

    let governanceAddress;

    if (isValidAddress(address)) {
        governanceAddress = address;
    } else {
        if (!contractConfig?.address) {
            throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
        }

        governanceAddress = contractConfig.address;
    }

    const target = options.target || chain.contracts.AxelarGateway?.address;

    if (!isValidAddress(target)) {
        throw new Error(`Missing target address.`);
    }

    if (!isValidDecimal(nativeValue)) {
        throw new Error(`Invalid native value: ${nativeValue}`);
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    printInfo('Chain', chain.name);

    const wallet = await getWallet(privateKey, provider, options);
    await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', governanceAddress);

    const governance = new Contract(governanceAddress, IGovernance.abi, wallet);

    const gasOptions = contractConfig?.gasOptions || chain?.gasOptions || {};
    printInfo('Gas options', JSON.stringify(gasOptions, null, 2));

    printInfo('Proposal Action', action);

    let gmpPayload;
    let calldata = options.calldata;

    switch (action) {
        case 'scheduleTimeLock': {
            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            if (!isValidTimeFormat(date)) {
                throw new Error(`Invalid ETA: ${date}. Please pass the eta in the format YYYY-MM-DDTHH:mm:ss`);
            }

            const eta = dateToEta(date);

            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            const minEta = currTime + contractConfig?.minimumTimeDelay;
            printInfo('Minimum eta', etaToDate(minEta));

            if (eta < minEta) {
                printWarn(`${date} is less than the minimum eta.`);
            }

            const existingProposalEta = await governance.getProposalEta(target, calldata, nativeValue);

            if (!existingProposalEta.eq(BigNumber.from(0))) {
                throw new Error(`Proposal already exists with eta: ${existingProposalEta}.`);
            }

            const commandType = 0;
            const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
            const values = [commandType, target, calldata, nativeValue, eta];

            gmpPayload = defaultAbiCoder.encode(types, values);

            break;
        }

        case 'cancelTimeLock': {
            const commandType = 1;

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            const proposalEta = await governance.getProposalEta(target, calldata, nativeValue);
            printInfo('Proposal eta', etaToDate(proposalEta));

            if (proposalEta.eq(BigNumber.from(0))) {
                throw new Error(`Proposal does not exist.`);
            }

            if (proposalEta <= currTime) {
                printWarn(`Proposal eta has already passed.`);
            }

            const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
            const values = [commandType, target, calldata, nativeValue, proposalEta];

            gmpPayload = defaultAbiCoder.encode(types, values);

            break;
        }

        case 'approveMultisig': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${action}`);
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            const commandType = 2;

            const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
            const values = [commandType, target, calldata, nativeValue, 0];

            gmpPayload = defaultAbiCoder.encode(types, values);

            break;
        }

        case 'cancelMultisig': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${action}`);
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            const commandType = 3;

            const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
            const values = [commandType, target, calldata, nativeValue, 0];

            gmpPayload = defaultAbiCoder.encode(types, values);

            break;
        }

        case 'executeProposal': {
            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            const eta = await governance.getTimeLock(proposalHash);

            if (eta.eq(0)) {
                throw new Error('Proposal does not exist.');
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            printInfo('Proposal ETA', etaToDate(eta));

            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            if (currTime < eta) {
                throw new Error(`TimeLock proposal is not yet eligible for execution.`);
            }

            if (prompt('Proceed with executing this proposal?', yes)) {
                throw new Error('Proposal execution cancelled.');
            }

            const tx = await governance.executeProposal(target, calldata, nativeValue, { gasLimit: 1e6 });
            printInfo('Proposal execution tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, governance, 'ProposalExecuted');

            if (!eventEmitted) {
                throw new Error('Proposal execution failed.');
            }

            printInfo('Proposal executed.');

            break;
        }

        case 'executeMultisigProposal': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${action}`);
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            const isApproved = await governance.multisigApprovals(proposalHash);

            if (!isApproved) {
                throw new Error('Multisig proposal has not been approved.');
            }

            const isSigner = await governance.isSigner(wallet.address);

            if (!isSigner) {
                throw new Error(`Caller is not a valid signer address: ${wallet.address}`);
            }

            const executeInterface = new Interface(governance.interface.fragments);
            const executeCalldata = executeInterface.encodeFunctionData('executeMultisigProposal', [target, calldata, nativeValue]);
            const topic = keccak256(executeCalldata);

            const hasSignerVoted = await governance.hasSignerVoted(wallet.address, topic);

            if (hasSignerVoted) {
                throw new Error(`Signer has already voted: ${wallet.address}`);
            }

            const signerVoteCount = await governance.getSignerVotesCount(topic);
            printInfo(`${signerVoteCount} signers have already voted.`);

            let receipt;

            try {
                const tx = await governance.executeMultisigProposal(target, calldata, nativeValue, gasOptions);
                receipt = await tx.wait();
            } catch (error) {
                printError(error);
            }

            const eventEmitted = wasEventEmitted(receipt, governance, 'MultisigExecuted');

            if (!eventEmitted) {
                throw new Error('Multisig proposal execution failed.');
            }

            printInfo('Multisig proposal executed.');

            break;
        }

        case 'gatewayUpgrade': {
            if (contractName === 'AxelarServiceGovernance') {
                throw new Error(`Invalid governance action for AxelarServiceGovernance: ${action}`);
            }

            if (!isValidTimeFormat(date)) {
                throw new Error(`Invalid ETA: ${date}. Please pass the eta in the format YYYY-MM-DDTHH:mm:ss`);
            }

            const eta = dateToEta(date);

            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            const minEta = currTime + contractConfig?.minimumTimeDelay;
            printInfo('Minimum eta', etaToDate(minEta));

            if (eta < minEta) {
                printWarn(`${date} is less than the minimum eta.`);
            }

            const implementation = options.implementation || chain.contracts.AxelarGateway?.implementation;

            if (!isValidAddress(implementation)) {
                throw new Error(`Invalid new gateway implementation address: ${implementation}`);
            }

            const gateway = new Contract(target, IGateway.abi, wallet);

            printInfo('Current gateway implementation', await gateway.implementation());
            printInfo('New gateway implementation', implementation);

            const newGatewayImplementationCodeHash = await getBytecodeHash(implementation, chain.name, provider);
            printInfo('New gateway implementation code hash', newGatewayImplementationCodeHash);

            const currGovernance = await gateway.governance();
            const currMintLimiter = await gateway.mintLimiter();

            if (currGovernance !== governance.address) {
                printWarn(`Gateway governor ${currGovernance} does not match governance contract: ${governance.address}`);
            }

            let newGovernance = options.newGovernance;

            if (contracts.AxelarGateway?.governance === currGovernance) {
                newGovernance = contracts.AxelarGateway?.governance;
            } else {
                newGovernance = '0x';
            }

            let newMintLimiter = options.newMintLimiter;

            if (contracts.AxelarGateway?.mintLimiter === currMintLimiter) {
                newMintLimiter = contracts.AxelarGateway?.mintLimiter;
            } else {
                newMintLimiter = '0x';
            }

            let setupParams = '0x';

            if (newGovernance !== '0x' || newMintLimiter !== '0x') {
                setupParams = defaultAbiCoder.encode(['address', 'address', 'bytes'], [newGovernance, newMintLimiter, '0x']);
            }

            printInfo('Setup Params for upgrading AxelarGateway', setupParams);

            calldata = gateway.interface.encodeFunctionData('upgrade', [
                implementation,
                newGatewayImplementationCodeHash,
                setupParams,
            ]);

            const commandType = 0;
            const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
            const values = [commandType, target, calldata, nativeValue, eta];

            gmpPayload = defaultAbiCoder.encode(types, values);
            const proposalEta = await governance.getProposalEta(target, calldata, nativeValue);

            if (!BigNumber.from(proposalEta).eq(0)) {
                printWarn('The proposal already exixts', etaToDate(proposalEta));
            }

            break;
        }

        case 'getProposalEta': {
            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            const eta = await governance.getTimeLock(proposalHash);

            if (eta.eq(0)) {
                printWarn('Proposal does not exist.');
            }

            printInfo('Proposal ETA', etaToDate(eta));

            break;
        }

        default: {
            throw new Error(`Unknown governance action ${action}`);
        }
    }

    if (gmpPayload) {
        printInfo('Destination chain', chain.name);
        printInfo('Destination governance address', governanceAddress);
        printInfo('GMP payload', gmpPayload);
        printInfo('Target contract', target);
        printInfo('Target calldata', calldata);
        printInfo('Native value', nativeValue);
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

const program = new Command();

program.name('governance').description('Script to manage interchain governance actions');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(
    new Option('-c, --contractName <contractName>', 'contract name')
        .choices(['InterchainGovernance', 'AxelarServiceGovernance'])
        .default('InterchainGovernance'),
);
program.addOption(new Option('-a, --address <address>', 'override address'));
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(
    new Option('--action <action>', 'governance action').choices([
        'scheduleTimeLock',
        'cancelTimeLock',
        'approveMultisig',
        'cancelMultisig',
        'executeProposal',
        'executeMultisigProposal',
        'gatewayUpgrade',
        'getProposalEta',
    ]),
);
program.addOption(new Option('--newGovernance <governance>', 'governance address').env('GOVERNANCE'));
program.addOption(new Option('--newMintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'));
program.addOption(new Option('--target <target>', 'governance execution target'));
program.addOption(new Option('--calldata <calldata>', 'calldata'));
program.addOption(new Option('--nativeValue <nativeValue>', 'nativeValue').default(0));
program.addOption(new Option('--date <date>', 'proposal activation date'));
program.addOption(new Option('--implementation <implementation>', 'new gateway implementation'));

program.action((options) => {
    main(options);
});

program.parse();
