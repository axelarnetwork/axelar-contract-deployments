'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { defaultAbiCoder, keccak256, Interface },
    Contract,
    BigNumber,
} = ethers;
const readlineSync = require('readline-sync');
const { Command, Option } = require('commander');
const {
    printInfo,
    printWalletInfo,
    loadConfig,
    isNumber,
    isValidTimeFormat,
    etaToUnixTimestamp,
    unixTimestampToEta,
    getCurrentTimeInSeconds,
    wasEventEmitted,
    printWarn,
    printError,
    getBytecodeHash,
    isValidAddress,
} = require('./utils');
const IGovernance = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarServiceGovernance.json');
const IGateway = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGateway.json');

async function processCommand(options, chain) {
    const {
        contractName,
        address,
        newGovernance,
        newMintLimiter,
        governanceAction,
        calldata,
        nativeValue,
        eta,
        implementation,
        privateKey,
        yes,
    } = options;

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

    const target = chain.contracts.AxelarGateway?.address;

    if (!isValidAddress(target)) {
        throw new Error(`Missing AxelarGateway address in the chain info.`);
    }

    if (!isNumber(parseFloat(nativeValue))) {
        throw new Error(`Invalid native value: ${nativeValue}`);
    }

    if (!isValidTimeFormat(eta)) {
        throw new Error(`Invalid ETA: ${eta}. Please pass the eta in the format YYYY-MM-DDTHH:mm:ss`);
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Contract name', contractName);

    const governanceContract = new Contract(governanceAddress, IGovernance.abi, wallet);

    const gasOptions = contractConfig?.gasOptions || chain?.gasOptions || {};
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    printInfo('Proposal Action', governanceAction);

    const unixEta = etaToUnixTimestamp(eta);

    const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
    const values = [0, target, calldata, nativeValue, unixEta];

    let gmpPayload;

    switch (governanceAction) {
        case 'scheduleTimeLock': {
            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${governanceAction}`);
            }

            if (unixEta < getCurrentTimeInSeconds() + contractConfig?.minimumTimeDelay && !yes) {
                printWarn(`${eta} is less than the minimum eta.`);
                const answer = readlineSync.question(`Proceed with ${governanceAction}?`);
                if (answer !== 'y') return;
            }

            gmpPayload = defaultAbiCoder.encode(types, values);

            printInfo(`Destination chain: ${chain.name}\nDestination governance address: ${governanceAddress}\nGMP payload: ${gmpPayload}`);

            break;
        }

        case 'cancelTimeLock': {
            const commandType = 1;

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${governanceAction}`);
            }

            if (unixEta < getCurrentTimeInSeconds() && !yes) {
                printWarn(`${eta} has already passed.`);
                const answer = readlineSync.question(`Proceed with ${governanceAction}?`);
                if (answer !== 'y') return;
            }

            const proposalEta = await governanceContract.getProposalEta(target, calldata, nativeValue);

            if (proposalEta.eq(BigNumber.from(0))) {
                throw new Error(`Proposal does not exist.`);
            }

            values[0] = commandType;
            gmpPayload = defaultAbiCoder.encode(types, values);

            printInfo(`Destination chain: ${chain.name}\nDestination governance address: ${governanceAddress}\nGMP payload: ${gmpPayload}`);

            break;
        }

        case 'approveMultisig': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${governanceAction}`);
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${governanceAction}`);
            }

            const commandType = 2;

            values[0] = commandType;
            gmpPayload = defaultAbiCoder.encode(types, values);

            printInfo(`Destination chain: ${chain.name}\nDestination governance address: ${governanceAddress}\nGMP payload: ${gmpPayload}`);

            break;
        }

        case 'cancelMultisig': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${governanceAction}`);
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${governanceAction}`);
            }

            const commandType = 3;

            values[0] = commandType;
            gmpPayload = defaultAbiCoder.encode(types, values);

            printInfo(`Destination chain: ${chain.name}\nDestination governance address: ${governanceAddress}\nGMP payload: ${gmpPayload}`);

            break;
        }

        case 'executeProposal': {
            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            const minimumEta = await governanceContract.getTimeLock(proposalHash);

            if (minimumEta === 0) {
                throw new Error('Proposal does not exist.');
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${governanceAction}`);
            }

            if (getCurrentTimeInSeconds() < minimumEta) {
                throw new Error(`TimeLock proposal is not yet eligible for execution.`);
            }

            let receipt;

            try {
                const tx = await governanceContract.executeProposal(target, calldata, nativeValue, gasOptions);
                receipt = tx.wait();
            } catch (error) {
                printError(error);
            }

            const eventEmitted = wasEventEmitted(receipt, governanceContract, 'ProposalExecuted');

            if (!eventEmitted) {
                throw new Error('Proposal execution failed.');
            }

            printInfo('Proposal executed.');

            break;
        }

        case 'executeMultisigProposal': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${governanceAction}`);
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${governanceAction}`);
            }

            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            const isApproved = await governanceContract.multisigApprovals(proposalHash);

            if (!isApproved) {
                throw new Error('Multisig proposal has not been approved.');
            }

            const isSigner = await governanceContract.isSigner(wallet.address);

            if (!isSigner) {
                throw new Error(`Caller is not a valid signer address: ${wallet.address}`);
            }

            const executeInterface = new Interface(governanceContract.interface.fragments);
            const executeCalldata = executeInterface.encodeFunctionData('executeMultisigProposal', [target, calldata, nativeValue]);
            const topic = keccak256(executeCalldata);

            const hasSignerVoted = await governanceContract.hasSignerVoted(wallet.address, topic);

            if (hasSignerVoted) {
                throw new Error(`Signer has already voted: ${wallet.address}`);
            }

            const signerVoteCount = await governanceContract.getSignerVotesCount(topic);
            printInfo(`${signerVoteCount} signers have already voted.`);

            let receipt;

            try {
                const tx = await governanceContract.executeMultisigProposal(target, calldata, nativeValue, gasOptions);
                receipt = await tx.wait();
            } catch (error) {
                printError(error);
            }

            const eventEmitted = wasEventEmitted(receipt, governanceContract, 'MultisigExecuted');

            if (!eventEmitted) {
                throw new Error('Multisig proposal execution failed.');
            }

            printInfo('Multisig proposal executed.');

            break;
        }

        case 'gatewayUpgrade': {
            if (contractName === 'AxelarServiceGovernance') {
                throw new Error(`Invalid governance action for AxelarServiceGovernance: ${governanceAction}`);
            }

            if (unixEta < getCurrentTimeInSeconds() + contractConfig?.minimumTimeDelay && !yes) {
                printWarn(`${eta} is less than the minimum eta.`);
                const answer = readlineSync.question(`Proceed with ${governanceAction}?`);
                if (answer !== 'y') return;
            }

            if (!isValidAddress(implementation)) {
                throw new Error(`Invalid new gateway implementation address: ${implementation}`);
            }

            const gatewayContract = new Contract(target, IGateway.abi, wallet);
            const targetInterface = new Interface(gatewayContract.interface.fragments);

            const implementationCode = await provider.getCode(implementation);

            if (implementationCode === '0x') {
                printWarn(`There is no code deployed at ${implementation}`);
                const answer = readlineSync.question(`Proceed with ${governanceAction}?`);
                if (answer !== 'y') return;
            }

            const newGatewayImplementationCodeHash = await getBytecodeHash(implementation, chain.name, provider);

            const governance = newGovernance || contracts.AxelarGateway?.governance || undefined;
            const mintLimiter = newMintLimiter || contracts.AxelarGateway?.mintLimiter || undefined;
            let setupParams;

            if (governance && mintLimiter) {
                setupParams = defaultAbiCoder.encode(['address', 'address', 'bytes'], [governance, mintLimiter, '0x']);
            } else {
                setupParams = '0x';
            }

            printInfo('Setup Params for upgrading AxelarGateway', setupParams);

            const upgradeCalldata = targetInterface.encodeFunctionData('upgrade', [
                implementation,
                newGatewayImplementationCodeHash,
                setupParams,
            ]);

            values[2] = upgradeCalldata;

            gmpPayload = defaultAbiCoder.encode(types, values);
            const proposalEta = await governanceContract.getProposalEta(target, upgradeCalldata, nativeValue);

            if (BigNumber.from(proposalEta).gt(0)) {
                printWarn("The eta for this proposal already exixts and it's value is", proposalEta);
            }

            printInfo(`Destination chain: ${chain.name}\nDestination governance address: ${governanceAddress}\nGMP payload: ${gmpPayload}`);

            break;
        }

        case 'getProposalEta': {
            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${governanceAction}`);
            }

            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            const minimumEta = await governanceContract.getTimeLock(proposalHash);

            if (minimumEta === 0) {
                throw new Error('Proposal does not exist.');
            }

            printInfo(`Proposal eta: ${unixTimestampToEta(minimumEta)}`);

            break;
        }

        default: {
            throw new Error(`Unknown governance action ${governanceAction}`);
        }
    }
}

async function main(options) {
    const config = loadConfig(options.env);

    const chain = options.destinationChain;

    if (config.chains[chain.toLowerCase()] === undefined) {
        throw new Error(`Destination chain ${chain} is not defined in the info file`);
    }

    await processCommand(options, config.chains[chain.toLowerCase()]);
}

const program = new Command();

program.name('governance-script').description('Script to manage interchain governance actions');

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
program.addOption(new Option('-a, --address <address>', 'override address').makeOptionMandatory(false));
program.addOption(new Option('-n, --destinationChain <destinationChain>', 'destination chain').makeOptionMandatory(true));
program.addOption(
    new Option('-g, --governanceAction <governanceAction>', 'governance action')
        .choices([
            'scheduleTimeLock',
            'cancelTimeLock',
            'approveMultisig',
            'cancelMultisig',
            'executeProposal',
            'executeMultisigProposal',
            'gatewayUpgrade',
            'getProposalEta',
        ])
        .default('scheduleTimeLock'),
);
program.addOption(new Option('--newGovernance <governance>', 'governance address').env('GOVERNANCE'));
program.addOption(new Option('--newMintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'));
program.addOption(new Option('-d, --calldata <calldata>', 'calldata').makeOptionMandatory(false));
program.addOption(new Option('-v, --nativeValue <nativeValue>', 'nativeValue').makeOptionMandatory(false).default(0));
program.addOption(new Option('-t, --eta <eta>', 'eta').makeOptionMandatory(false).default('0'));
program.addOption(new Option('--implementation <implementation>', 'new gateway implementation').makeOptionMandatory(false));
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

program.action((options) => {
    main(options);
});

program.parse();
