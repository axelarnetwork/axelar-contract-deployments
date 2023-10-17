'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { defaultAbiCoder, keccak256, Interface, parseEther, arrayify },
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
const IInterchainTokenService = require('@axelar-network/interchain-token-service/dist/interfaces/IInterchainTokenService.sol');

async function getGatewaySetupParams(governance, gateway, contracts, options) {
    const currGovernance = await gateway.governance();
    const currMintLimiter = await gateway.mintLimiter();

    if (currGovernance !== governance.address) {
        printWarn(`Gateway governor ${currGovernance} does not match governance contract: ${governance.address}`);
    }

    let newGovernance = options.newGovernance || contracts.AxelarGateway?.governance;

    if (newGovernance === currGovernance) {
        newGovernance = '0x';
    }

    let newMintLimiter = options.newMintLimiter || contracts.AxelarGateway?.mintLimiter;

    if (newMintLimiter === `${currMintLimiter}`) {
        newMintLimiter = '0x';
    }

    let setupParams = '0x';

    if (newGovernance !== '0x' || newMintLimiter !== '0x') {
        setupParams = defaultAbiCoder.encode(['address', 'address', 'bytes'], [newGovernance, newMintLimiter, '0x']);
    }

    return setupParams;
}

async function getITSSetupParams(options) {
    let setupParams;
    const newOperator = options.newOperator;

    if (newOperator) {
        if (!isValidAddress(newOperator)) {
            throw new Error(`Invalid new operator address: ${newOperator}`);
        }

        setupParams = arrayify(newOperator);
    } else {
        setupParams = '0x';
    }

    return setupParams;
}

async function processCommand(_, chain, options) {
    const { contractName, address, action, date, privateKey, yes } = options;

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

    let target = options.target || chain.contracts.AxelarGateway?.address;
    let nativeValue = options.nativeValue;

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

    const gasOptions = contractConfig?.gasOptions || chain?.gasOptions || { gasLimit: 5e6 };
    printInfo('Gas options', JSON.stringify(gasOptions, null, 2));

    printInfo('Proposal Action', action);

    let gmpPayload;
    let title = `Governance proposal for chain ${chain.name}`;
    let description = `This proposal submits a governance command for chain ${chain.name}`;
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
                printWarn(`Proposal does not exist.`);
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
            if (options.proposal) {
                printInfo('Decoding proposal to get governance data');

                [_, target, calldata, nativeValue, _] = defaultAbiCoder.decode(
                    ['uint256', 'address', 'bytes', 'uint256', 'uint256'],
                    options.proposal,
                );
            }

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

            const tx = await governance.executeProposal(target, calldata, nativeValue, gasOptions);
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

            printInfo('Time difference between current time and eta', etaToDate(eta - currTime));

            const implementation = options.implementation || chain.contracts.AxelarGateway?.implementation;

            if (!isValidAddress(implementation)) {
                throw new Error(`Invalid new gateway implementation address: ${implementation}`);
            }

            const gateway = new Contract(target, IGateway.abi, wallet);

            printInfo('Current gateway implementation', await gateway.implementation());
            printInfo('New gateway implementation', implementation);

            const newGatewayImplementationCodeHash = await getBytecodeHash(implementation, chain.name, provider);
            printInfo('New gateway implementation code hash', newGatewayImplementationCodeHash);

            const setupParams = await getGatewaySetupParams(governance, gateway, contracts, options);

            printInfo('Setup Params for upgrading AxelarGateway', setupParams);

            calldata = gateway.interface.encodeFunctionData('upgrade', [implementation, newGatewayImplementationCodeHash, setupParams]);

            const commandType = 0;
            const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
            const values = [commandType, target, calldata, nativeValue, eta];

            gmpPayload = defaultAbiCoder.encode(types, values);
            const proposalEta = await governance.getProposalEta(target, calldata, nativeValue);

            if (!BigNumber.from(proposalEta).eq(0)) {
                printWarn('The proposal already exixts', etaToDate(proposalEta));
            }

            title = `Chain ${chain.name} gateway upgrade proposal`;
            description = `This proposal upgrades the gateway contract ${gateway.address} on chain ${chain.name} to a new implementation contract ${implementation}`;

            break;
        }

        case 'executeUpgrade': {
            target = contracts.AxelarGateway?.address;
            const gateway = new Contract(target, IGateway.abi, wallet);
            const implementation = options.implementation || chain.contracts.AxelarGateway?.implementation;
            const implementationCodehash = chain.contracts.AxelarGateway?.implementationCodehash;
            printInfo('New gateway implementation code hash', implementationCodehash);

            if (!isValidAddress(implementation)) {
                throw new Error(`Invalid new gateway implementation address: ${implementation}`);
            }

            const setupParams = await getGatewaySetupParams(governance, gateway, contracts, options);

            printInfo('Setup Params for upgrading AxelarGateway', setupParams);

            calldata = gateway.interface.encodeFunctionData('upgrade', [implementation, implementationCodehash, setupParams]);

            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            const eta = await governance.getTimeLock(proposalHash);

            if (eta.eq(0)) {
                printError('Proposal does not exist.');
                return;
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            printInfo('Proposal ETA', etaToDate(eta));

            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            if (currTime < eta) {
                throw new Error(`Upgrade proposal is not yet eligible for execution.`);
            }

            if (prompt('Proceed with executing this proposal?', yes)) {
                throw new Error('Proposal execution cancelled.');
            }

            const tx = await governance.executeProposal(target, calldata, nativeValue, gasOptions);
            printInfo('Proposal execution tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, governance, 'ProposalExecuted');

            if (!eventEmitted) {
                throw new Error('Proposal execution failed.');
            }

            printInfo('Proposal executed.');

            break;
        }

        case 'ITSUpgrade': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${action}`);
            }

            const isTimeLock = options.isTimeLock;
            let eta = 0;

            if (isTimeLock) {
                eta = dateToEta(date);

                const currTime = getCurrentTimeInSeconds();
                printInfo('Current time', etaToDate(currTime));

                const minEta = currTime + contractConfig?.minimumTimeDelay;
                printInfo('Minimum eta', etaToDate(minEta));

                if (eta < minEta) {
                    printWarn(`${date} is less than the minimum eta.`);
                }

                printInfo('Time difference between current time and eta', etaToDate(eta - currTime));
            }

            const implementation = options.implementation || chain.contracts.InterchainTokenService?.implementation;

            if (!isValidAddress(implementation)) {
                throw new Error(`Invalid new ITS implementation address: ${implementation}`);
            }

            const ITS = new Contract(target, IInterchainTokenService.abi, wallet);

            printInfo('Current ITS implementation', await ITS.implementation());
            printInfo('New ITS implementation', implementation);

            const newITSImplementationCodeHash = await getBytecodeHash(implementation, chain.name, provider);
            printInfo('New ITS implementation code hash', newITSImplementationCodeHash);

            const setupParams = await getITSSetupParams(governance, ITS, contracts, options);

            printInfo('Setup Params for upgrading AxelarGateway', setupParams);

            calldata = ITS.interface.encodeFunctionData('upgrade', [implementation, newITSImplementationCodeHash, setupParams]);

            const commandType = 0;
            const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
            const values = [commandType, target, calldata, nativeValue, eta];

            gmpPayload = defaultAbiCoder.encode(types, values);
            const proposalEta = await governance.getProposalEta(target, calldata, nativeValue);

            if (!BigNumber.from(proposalEta).eq(0)) {
                printWarn('The proposal already exixts', etaToDate(proposalEta));
            }

            title = `Chain ${chain.name} ITS upgrade proposal`;
            description = `This proposal upgrades the ITS contract ${ITS.address} on chain ${chain.name} to a new implementation contract ${implementation}`;

            break;
        }

        case 'executeITSUpgrade': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${action}`);
            }

            target = contracts.InterchainTokenService?.address;
            const ITS = new Contract(target, IInterchainTokenService.abi, wallet);
            const implementation = options.implementation || chain.contracts.InterchainTokenService?.implementation;
            const implementationCodehash = chain.contracts.InterchainTokenService?.implementationCodehash;
            printInfo('New ITS implementation code hash', implementationCodehash);

            if (!isValidAddress(implementation)) {
                throw new Error(`Invalid new ITS implementation address: ${implementation}`);
            }

            const setupParams = await getITSSetupParams(options);

            printInfo('Setup Params for upgrading ITS', setupParams);

            calldata = ITS.interface.encodeFunctionData('upgrade', [implementation, implementationCodehash, setupParams]);

            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            const eta = await governance.getTimeLock(proposalHash);

            const isTimeLock = options.isTimelock;

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            let tx;

            if (isTimeLock) {
                if (eta.eq(0)) {
                    printError('TimeLock proposal does not exist.');
                    return;
                }

                printInfo('Proposal ETA', etaToDate(eta));

                const currTime = getCurrentTimeInSeconds();
                printInfo('Current time', etaToDate(currTime));

                if (currTime < eta) {
                    throw new Error(`Upgrade timeLock proposal is not yet eligible for execution.`);
                }

                if (prompt('Proceed with executing this proposal?', yes)) {
                    throw new Error('Proposal execution cancelled.');
                }

                tx = await governance.executeProposal(target, calldata, nativeValue, gasOptions);
            } else {
                const isApproved = await governance.multisigApprovals(proposalHash);

                if (!isApproved) {
                    throw new Error('Upgrade multisig proposal has not been approved.');
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

                if (prompt('Proceed with executing this proposal?', yes)) {
                    throw new Error('Proposal execution cancelled.');
                }

                tx = await governance.executeMultisigProposal(target, calldata, nativeValue, gasOptions);
            }

            printInfo('Proposal execution tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, governance, 'ProposalExecuted');

            if (!eventEmitted) {
                throw new Error('Proposal execution failed.');
            }

            printInfo('Proposal executed.');

            break;
        }

        case 'withdraw': {
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

            if (!isValidAddress(options.target)) {
                throw new Error(`Invalid target address: ${options.target}`);
            }

            if (!isValidDecimal(options.amount)) {
                throw new Error(`Invalid withdraw amount: ${options.amount}`);
            }

            const amount = parseEther(options.amount);
            calldata = governance.interface.encodeFunctionData('withdraw', [options.target, amount]);
            target = governance.address;

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
        const payloadBase64 = Buffer.from(`${gmpPayload}`.slice(2), 'hex').toString('base64');

        printInfo('Destination chain', chain.name);
        printInfo('Destination governance address', governanceAddress);
        printInfo('Governance call contract payload', gmpPayload);
        printInfo('Governance payload hash', keccak256(gmpPayload));
        printInfo('Governance call target', target);
        printInfo('Governance call data', calldata);
        printInfo('Governance native value', nativeValue || '0');
        printInfo('Date', date);

        const proposal = {
            title,
            description,
            contract_calls: [
                {
                    chain: chain.id,
                    contract_address: governanceAddress,
                    payload: payloadBase64,
                },
            ],
        };

        // printInfo('Proposal', JSON.stringify(proposal, null, 2));
        console.log(JSON.stringify(proposal.contract_calls[0]));
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
program.addOption(new Option('--skipChains <skipChains>', 'chains to skip over'));
program.addOption(
    new Option('--action <action>', 'governance action').choices([
        'scheduleTimeLock',
        'cancelTimeLock',
        'approveMultisig',
        'cancelMultisig',
        'executeProposal',
        'executeMultisigProposal',
        'gatewayUpgrade',
        'executeUpgrade',
        'ITSUpgrade',
        'executeITSUpgrade',
        'withdraw',
        'getProposalEta',
    ]),
);
program.addOption(new Option('--newGovernance <governance>', 'governance address').env('GOVERNANCE'));
program.addOption(new Option('--newMintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'));
program.addOption(new Option('--newOperator <operator>', 'new operator address'));
program.addOption(new Option('--isTimeLock <isTimeLock>', 'ITS upgrade proposal type').default(false));
program.addOption(new Option('--target <target>', 'governance execution target'));
program.addOption(new Option('--calldata <calldata>', 'calldata'));
program.addOption(new Option('--nativeValue <nativeValue>', 'nativeValue').default(0));
program.addOption(new Option('--proposal <proposal>', 'governance proposal payload'));
program.addOption(new Option('--amount <amount>', 'withdraw amount'));
program.addOption(new Option('--date <date>', 'proposal activation date'));
program.addOption(new Option('--implementation <implementation>', 'new gateway implementation'));

program.action((options) => {
    main(options);
});

program.parse();
