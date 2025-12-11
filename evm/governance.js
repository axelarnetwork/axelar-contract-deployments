'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { defaultAbiCoder, keccak256, parseEther },
    Contract,
    BigNumber,
    constants: { AddressZero },
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    getGasOptions,
    printWalletInfo,
    dateToEta,
    etaToDate,
    getCurrentTimeInSeconds,
    createGMPProposalJSON,
    handleTransactionWithEvent,
    printWarn,
    getBytecodeHash,
    getGovernanceAddress,
    mainProcessor,
    prompt,
    writeJSON,
    validateParameters,
    getContractJSON,
} = require('./utils.js');
const { addBaseOptions, addOptionsToCommands } = require('./cli-utils');
const { getWallet } = require('./sign-utils.js');
const { submitCallContracts } = require('../cosmwasm/utils');
const { mainProcessor: cosmwasmMainProcessor } = require('../cosmwasm/processor');
const IAxelarServiceGovernance = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarServiceGovernance.json');
const AxelarGateway = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/AxelarGateway.sol/AxelarGateway.json');
const IUpgradable = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IUpgradable.json');
const ProposalType = {
    ScheduleTimelock: 0,
    CancelTimelock: 1,
    ApproveOperator: 2,
    CancelOperator: 3,
};

function addGovernanceOptions(program) {
    program.addOption(new Option('--nativeValue <nativeValue>', 'native value').default('0'));
    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').env('MNEMONIC'));

    return program;
}

async function getSetupParams(governance, targetContractName, target, contracts, wallet, options) {
    let setupParams = '0x';

    switch (targetContractName) {
        case 'AxelarGateway': {
            const gateway = new Contract(target, AxelarGateway.abi, wallet);
            const currGovernance = await gateway.governance();
            const currMintLimiter = await gateway.mintLimiter();

            if (currGovernance !== governance.address) {
                printWarn(`Gateway governor ${currGovernance} does not match governance contract: ${governance.address}`);
            }

            let newGovernance = options.newGovernance || contracts.InterchainGovernance?.address || AddressZero;
            if (newGovernance === currGovernance) {
                newGovernance = AddressZero;
            }

            let newMintLimiter = options.newMintLimiter || contracts.Multisig?.address || AddressZero;
            if (newMintLimiter === currMintLimiter) {
                newMintLimiter = AddressZero;
            }

            if (newGovernance !== AddressZero || newMintLimiter !== AddressZero) {
                setupParams = defaultAbiCoder.encode(['address', 'address', 'bytes'], [newGovernance, newMintLimiter, '0x']);
            }

            break;
        }

        case 'InterchainTokenService':
        case 'InterchainTokenFactory': {
            break;
        }
    }

    return setupParams;
}

async function getProposalCalldata(governance, chain, wallet, action, options) {
    const contractName = options.contractName || 'AxelarServiceGovernance';
    const targetContractName = options.targetContractName;
    let target = options.target || chain.contracts[targetContractName]?.address;

    let calldata;
    const provider = getDefaultProvider(chain.rpc);
    let title = `Governance proposal for chain ${chain.name}`;
    let description = `This proposal submits a governance command for chain ${chain.name}`;

    switch (action) {
        case 'raw': {
            calldata = options.calldata;
            break;
        }

        case 'upgrade': {
            const implementation =
                options.implementation ||
                (targetContractName === 'AxelarGateway' ? chain.contracts[targetContractName]?.implementation : '');

            validateParameters({
                isValidAddress: { implementation },
            });

            const upgradable = new Contract(target, IUpgradable.abi, wallet);
            const currImplementation = await upgradable.implementation();

            printInfo('Current implementation', currImplementation);
            printInfo('New implementation', implementation);

            if (currImplementation === implementation) {
                printWarn(`Current implementation ${currImplementation} matches new implementation ${implementation}`);
            }

            const newImplementationCodeHash = await getBytecodeHash(implementation, chain.axelarId, provider);
            printInfo('New implementation code hash', newImplementationCodeHash);

            const setupParams = await getSetupParams(governance, targetContractName, target, chain.contracts, wallet, options);
            printInfo('Setup Params for upgrading', setupParams);

            calldata = upgradable.interface.encodeFunctionData('upgrade', [implementation, newImplementationCodeHash, setupParams]);

            title = `Chain ${chain.name} ${action} proposal`;
            description = `This proposal ${action}s the contract ${target} on chain ${chain.name} to a new implementation contract ${implementation}`;

            break;
        }

        case 'transferOperatorship': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: transferOperatorship`);
            }

            if (options.newOperator) {
                const newOperator = options.newOperator;

                validateParameters({
                    isValidAddress: { newOperator },
                });

                calldata = governance.interface.encodeFunctionData('transferOperatorship', [newOperator]);
                title = `Chain ${chain.name} transfer operatorship`;
                description = `Transfers operatorship of AxelarServiceGovernance to ${newOperator} on chain ${chain.name}`;
            } else {
                calldata = options.calldata;
            }
            target = governance.address;

            break;
        }

        case 'transferGovernance': {
            const newGovernance = options.newGovernance || chain.contracts.InterchainGovernance?.address;

            validateParameters({
                isValidAddress: { newGovernance },
            });

            const gateway = new Contract(target, AxelarGateway.abi, wallet);
            const currGovernance = await gateway.governance();

            printInfo('Current gateway governance', currGovernance);
            printInfo('New gateway governance', newGovernance);

            if (currGovernance !== governance.address) {
                printWarn(`Gateway governor ${currGovernance} does not match governance contract: ${governance.address}`);
            }

            calldata = gateway.interface.encodeFunctionData('transferGovernance', [newGovernance]);
            break;
        }

        case 'withdraw': {
            validateParameters({
                isValidDecimal: { amount: options.amount },
            });
            validateParameters({
                isValidDecimal: { amount: options.amount },
            });

            const amount = parseEther(options.amount);
            calldata = governance.interface.encodeFunctionData('withdraw', [options.target, amount]);
            target = governance.address;

            break;
        }

        default: {
            throw new Error(`Unknown governance action: ${action}`);
        }
    }

    validateParameters({
        isValidAddress: { target },
        isValidCalldata: { calldata },
    });

    printInfo('Governance target', target);
    printInfo('Governance calldata', calldata);

    return { target, calldata, title, description };
}

function encodeGovernanceProposal(commandType, target, calldata, nativeValue, eta) {
    const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
    const values = [commandType, target, calldata, nativeValue, eta];
    return defaultAbiCoder.encode(types, values);
}

function getProposalHash(target, calldata, nativeValue) {
    return keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
}

async function processCommand(_axelar, chain, _chains, action, options) {
    const { contractName, address, privateKey, args = [] } = options;

    const governanceAddress = getGovernanceAddress(chain, contractName, address);
    const provider = getDefaultProvider(chain.rpc);
    const wallet = await getWallet(privateKey, provider, options);
    await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', governanceAddress);

    const governance = new Contract(governanceAddress, IAxelarServiceGovernance.abi, wallet);
    const gasOptions = await getGasOptions(chain, options, contractName);

    let nativeValue = options.nativeValue || '0';
    validateParameters({
        isValidDecimal: { nativeValue },
    });
    nativeValue = nativeValue.toString();

    switch (action) {
        case 'eta': {
            let target = options.target;
            let calldata = options.calldata;

            if (options.proposal) {
                const decoded = defaultAbiCoder.decode(['uint256', 'address', 'bytes', 'uint256', 'uint256'], options.proposal);
                target = decoded[1];
                calldata = decoded[2];
                nativeValue = decoded[3].toString();
            }

            validateParameters({
                isValidAddress: { target },
                isValidCalldata: { calldata },
            });

            const proposalHash = getProposalHash(target, calldata, nativeValue);
            const eta = await governance.getTimeLock(proposalHash);

            if (eta.eq(0)) {
                printWarn('Proposal does not exist.');
            } else {
                printInfo('Proposal ETA', etaToDate(eta));
            }

            return null;
        }

        case 'schedule': {
            const [action, activationTime] = args;

            const { target, calldata } = await getProposalCalldata(governance, chain, wallet, action, options);

            validateParameters({
                isValidTimeFormat: { activationTime },
            });

            const eta = dateToEta(activationTime);
            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            const minEta = currTime + (await governance.minimumTimeLockDelay()).toNumber();
            printInfo('Minimum eta', etaToDate(minEta));

            if (eta < minEta) {
                printWarn(`${activationTime} is less than the minimum eta.`);
            }

            printInfo('Time difference between current time and eta', etaToDate(eta - currTime));

            const existingProposalEta = await governance.getProposalEta(target, calldata, nativeValue);
            if (!existingProposalEta.eq(BigNumber.from(0))) {
                throw new Error(`Proposal already exists with eta: ${existingProposalEta}.`);
            }

            const gmpPayload = encodeGovernanceProposal(ProposalType.ScheduleTimelock, target, calldata, nativeValue, eta);
            printInfo('Governance target (for eta/execute)', target);
            printInfo('Governance calldata (for eta/execute)', calldata);
            return createGMPProposalJSON(chain, governanceAddress, gmpPayload);
        }

        case 'cancel': {
            const [action] = args;

            const { target, calldata } = await getProposalCalldata(governance, chain, wallet, action, options);

            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            const eta = await governance.getProposalEta(target, calldata, nativeValue);
            printInfo('Proposal eta', etaToDate(eta));

            if (eta.eq(BigNumber.from(0))) {
                printWarn('Proposal does not exist.');
            }

            if (eta <= currTime) {
                printWarn('Proposal eta has already passed.');
            }

            const gmpPayload = encodeGovernanceProposal(ProposalType.CancelTimelock, target, calldata, nativeValue, eta);
            return createGMPProposalJSON(chain, governanceAddress, gmpPayload);
        }

        case 'schedule-operator': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: scheduleOperator`);
            }

            const [target, calldata, activationTime] = args;

            validateParameters({
                isValidAddress: { target },
                isValidCalldata: { calldata },
            });

            validateParameters({
                isValidTimeFormat: { activationTime },
            });

            const eta = dateToEta(activationTime);
            const gmpPayload = encodeGovernanceProposal(ProposalType.ApproveOperator, target, calldata, nativeValue, eta);
            return createGMPProposalJSON(chain, governanceAddress, gmpPayload);
        }

        case 'cancel-operator': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: cancelOperator`);
            }

            const [target, calldata] = args;

            validateParameters({
                isValidAddress: { target },
                isValidCalldata: { calldata },
            });

            const proposalHash = getProposalHash(target, calldata, nativeValue);
            const isApproved = await governance.isOperatorProposalApproved(target, calldata, nativeValue);

            if (!isApproved) {
                printWarn('Operator proposal is not approved.');
            }

            const gmpPayload = encodeGovernanceProposal(ProposalType.CancelOperator, target, calldata, nativeValue, 0);
            return createGMPProposalJSON(chain, governanceAddress, gmpPayload);
        }

        case 'submit': {
            const [action, commandId, activationTime] = args;

            const { target, calldata } = await getProposalCalldata(governance, chain, wallet, action, options);

            validateParameters({
                isKeccak256Hash: { commandId },
                isValidTimeFormat: { activationTime },
            });

            const eta = dateToEta(activationTime);
            const gmpPayload = encodeGovernanceProposal(ProposalType.ScheduleTimelock, target, calldata, nativeValue, eta);

            if (prompt('Proceed with submitting this proposal?', options.yes)) {
                throw new Error('Proposal submission cancelled.');
            }

            const contracts = chain.contracts;
            const tx = await governance.execute(
                commandId,
                contracts.InterchainGovernance.governanceChain,
                contracts.InterchainGovernance.governanceAddress,
                gmpPayload,
                gasOptions,
            );

            await handleTransactionWithEvent(tx, chain, governance, 'Proposal submission', 'ProposalScheduled');
            return null;
        }

        case 'submit-operator': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: submitOperator`);
            }

            const [target, calldata, commandId, activationTime] = args;

            validateParameters({
                isValidAddress: { target },
                isValidCalldata: { calldata },
            });

            validateParameters({
                isKeccak256Hash: { commandId },
                isValidTimeFormat: { activationTime },
            });

            const eta = dateToEta(activationTime);
            const gmpPayload = encodeGovernanceProposal(ProposalType.ApproveOperator, target, calldata, nativeValue, eta);

            if (prompt('Proceed with submitting this proposal?', options.yes)) {
                throw new Error('Proposal submission cancelled.');
            }

            const contracts = chain.contracts;
            const tx = await governance.execute(
                commandId,
                contracts.InterchainGovernance.governanceChain,
                contracts.InterchainGovernance.governanceAddress,
                gmpPayload,
                gasOptions,
            );

            await handleTransactionWithEvent(tx, chain, governance, 'Proposal submission', 'OperatorProposalApproved');
            return null;
        }

        case 'execute': {
            let target = options.target;
            let calldata = options.calldata;

            if (options.proposal) {
                printInfo('Decoding proposal to get governance data');
                const decoded = defaultAbiCoder.decode(['uint256', 'address', 'bytes', 'uint256', 'uint256'], options.proposal);
                target = decoded[1];
                calldata = decoded[2];
                nativeValue = decoded[3].toString();
            }

            validateParameters({
                isValidAddress: { target },
                isValidCalldata: { calldata },
            });

            const proposalHash = getProposalHash(target, calldata, nativeValue);
            const eta = await governance.getTimeLock(proposalHash);

            if (eta.eq(0)) {
                throw new Error('Proposal does not exist.');
            }

            printInfo('Proposal ETA', etaToDate(eta));

            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            if (currTime < eta.toNumber()) {
                throw new Error(`TimeLock proposal is not yet eligible for execution. ETA: ${etaToDate(eta)}`);
            }

            if (prompt('Proceed with executing this proposal?', options.yes)) {
                throw new Error('Proposal execution cancelled.');
            }

            if (nativeValue === '0') {
                printWarn('nativeValue is 0; no native token will be forwarded to the target call.');
            }

            const tx = await governance.executeProposal(target, calldata, nativeValue, gasOptions);
            await handleTransactionWithEvent(tx, chain, governance, 'Proposal execution', 'ProposalExecuted');

            printInfo('Proposal executed.');
            return null;
        }

        case 'execute-operator-proposal': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: execute-operator-proposal`);
            }

            const [target, calldata] = args;

            validateParameters({
                isValidAddress: { target },
                isValidCalldata: { calldata },
            });

            const isApproved = await governance.isOperatorProposalApproved(target, calldata, nativeValue);
            if (!isApproved) {
                throw new Error('Operator proposal is not approved. Submit (or wait for) approval before executing.');
            }

            if (prompt('Proceed with executing this operator proposal?', options.yes)) {
                throw new Error('Operator proposal execution cancelled.');
            }

            if (nativeValue === '0') {
                printWarn('nativeValue is 0; no native token will be forwarded to the target call.');
            }

            const tx = await governance.executeOperatorProposal(target, calldata, nativeValue, gasOptions);
            await handleTransactionWithEvent(tx, chain, governance, 'Operator proposal execution', 'OperatorProposalExecuted');
            return null;
        }

        case 'is-operator-approved': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: is-operator-approved`);
            }

            const [target, calldata] = args;

            validateParameters({
                isValidAddress: { target },
                isValidCalldata: { calldata },
            });

            const isApproved = await governance.isOperatorProposalApproved(target, calldata, nativeValue);
            printInfo('Operator proposal approved', isApproved);
            return null;
        }

        default: {
            throw new Error(`Unknown proposal action: ${action}`);
        }
    }
}

async function submitProposalToAxelar(proposal, options) {
    const submitFn = async (client, config, submitOptions, _args, fee) => {
        printInfo('Proposal details:');
        printInfo('Proposal title', proposal.title);
        printInfo('Proposal description', proposal.description);
        printInfo('Number of contract calls', proposal.contract_calls.length);
        printInfo('Contract calls', JSON.stringify(proposal.contract_calls, null, 2));

        printInfo('Submitting proposal to Axelar...');
        const proposalId = await submitCallContracts(client, config, submitOptions, proposal, fee);
        printInfo('Proposal submitted successfully! Proposal ID', proposalId);
        return proposalId;
    };

    const submitOptions = {
        env: options.env,
        mnemonic: options.mnemonic,
        contractName: 'Coordinator',
        chainName: 'axelar',
        title: proposal.title,
        description: proposal.description,
        yes: options.yes,
    };

    await cosmwasmMainProcessor(submitFn, submitOptions);
}

async function main(action, args, options) {
    options.args = args;
    const proposals = [];

    await mainProcessor(options, (axelar, chain, chains, options) => {
        return processCommand(axelar, chain, chains, action, options).then((proposal) => {
            if (proposal) {
                proposals.push(proposal);
            }
        });
    });

    if (proposals.length > 0) {
        const proposal = {
            title: 'Interchain Governance Proposal',
            description: 'Interchain Governance Proposal',
            contract_calls: proposals,
        };

        const proposalJSON = JSON.stringify(proposal, null, 2);

        printInfo('Proposal', proposalJSON);

        if (options.file) {
            writeJSON(proposal, options.file);
            printInfo('Proposal written to file', options.file);
        } else {
            if (!prompt('Proceed with submitting this proposal to Axelar?', options.yes)) {
                await submitProposalToAxelar(proposal, options);
            }
        }
    }
}

if (require.main === module) {
    const program = new Command();
    program.name('governance').description('Script to manage interchain governance actions');

    program
        .command('eta')
        .description('Get the ETA (estimated time of arrival) for a proposal')
        .addOption(new Option('--target <target>', 'target address (required if --proposal not provided)'))
        .addOption(new Option('--calldata <calldata>', 'call data (required if --proposal not provided)'))
        .addOption(new Option('--proposal <proposal>', 'governance proposal payload (alternative to target/calldata)'))
        .addOption(
            new Option('-c, --contractName <contractName>', 'contract name')
                .choices(['InterchainGovernance', 'AxelarServiceGovernance'])
                .default('AxelarServiceGovernance'),
        )
        .addOption(new Option('--targetContractName <targetContractName>', 'target contract name'))
        .action((options, cmd) => {
            if (!options.proposal && (!options.target || !options.calldata)) {
                throw new Error('Either --proposal or both --target and --calldata must be provided');
            }
            main(cmd.name(), [], options);
        });

    program
        .command('schedule')
        .description('Schedule a new timelock proposal')
        .argument('<action>', 'governance action (raw, upgrade, transferGovernance, transferOperatorship, withdraw)')
        .argument(
            '<activationTime>',
            'proposal activation time as UTC timestamp (YYYY-MM-DDTHH:mm:ss) or relative delay in seconds (numeric)',
        )
        .addOption(
            new Option('--targetContractName <targetContractName>', 'target contract name (required for upgrade, transferGovernance)'),
        )
        .addOption(new Option('--target <target>', 'governance execution target (required for raw action)'))
        .addOption(new Option('--calldata <calldata>', 'calldata (required for raw action)'))
        .addOption(new Option('--file <file>', 'file to write Axelar proposal JSON to'))
        .addOption(
            new Option('-c, --contractName <contractName>', 'contract name')
                .choices(['InterchainGovernance', 'AxelarServiceGovernance'])
                .default('AxelarServiceGovernance'),
        )
        .addOption(new Option('--newGovernance <governance>', 'governance address').env('GOVERNANCE'))
        .addOption(new Option('--newMintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'))
        .addOption(new Option('--newOperator <newOperator>', 'operator address').env('OPERATOR'))
        .addOption(new Option('--implementation <implementation>', 'new gateway implementation'))
        .addOption(new Option('--amount <amount>', 'withdraw amount'))
        .action((governanceAction, activationTime, options, cmd) => {
            main(cmd.name(), [governanceAction, activationTime], options);
        });

    program
        .command('cancel')
        .description('Cancel a scheduled timelock proposal')
        .argument('<action>', 'governance action (raw, upgrade, transferGovernance, transferOperatorship, withdraw)')
        .addOption(
            new Option('--targetContractName <targetContractName>', 'target contract name (required for upgrade, transferGovernance)'),
        )
        .addOption(new Option('--target <target>', 'governance execution target (required for raw action)'))
        .addOption(new Option('--calldata <calldata>', 'calldata (required for raw action)'))
        .addOption(new Option('--file <file>', 'file to write Axelar proposal JSON to'))
        .addOption(
            new Option('-c, --contractName <contractName>', 'contract name')
                .choices(['InterchainGovernance', 'AxelarServiceGovernance'])
                .default('AxelarServiceGovernance'),
        )
        .addOption(new Option('--newGovernance <governance>', 'governance address').env('GOVERNANCE'))
        .addOption(new Option('--newMintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'))
        .addOption(new Option('--newOperator <newOperator>', 'operator address').env('OPERATOR'))
        .addOption(new Option('--implementation <implementation>', 'new gateway implementation'))
        .addOption(new Option('--amount <amount>', 'withdraw amount'))
        .action((governanceAction, options, cmd) => {
            main(cmd.name(), [governanceAction], options);
        });

    program
        .command('execute')
        .description('Execute a scheduled proposal')
        .addOption(new Option('--target <target>', 'target address (required if --proposal not provided)'))
        .addOption(new Option('--calldata <calldata>', 'call data (required if --proposal not provided)'))
        .addOption(new Option('--proposal <proposal>', 'governance proposal payload (alternative to target/calldata)'))
        .addOption(
            new Option('-c, --contractName <contractName>', 'contract name')
                .choices(['InterchainGovernance', 'AxelarServiceGovernance'])
                .default('AxelarServiceGovernance'),
        )
        .addOption(new Option('--targetContractName <targetContractName>', 'target contract name'))
        .action((options, cmd) => {
            if (!options.proposal && (!options.target || !options.calldata)) {
                throw new Error('Either --proposal or both --target and --calldata must be provided');
            }
            main(cmd.name(), [], options);
        });

    program
        .command('schedule-operator')
        .description('Schedule an operator proposal (AxelarServiceGovernance only)')
        .argument('<target>', 'target address')
        .argument('<calldata>', 'call data')
        .argument(
            '<activationTime>',
            'proposal activation time as UTC timestamp (YYYY-MM-DDTHH:mm:ss) or relative delay in seconds (numeric)',
        )
        .addOption(new Option('--file <file>', 'file to write Axelar proposal JSON to'))
        .addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarServiceGovernance'))
        .action((target, calldata, activationTime, options, cmd) => {
            main(cmd.name(), [target, calldata, activationTime], options);
        });

    program
        .command('cancel-operator')
        .description('Cancel an operator proposal (AxelarServiceGovernance only)')
        .argument('<target>', 'target address')
        .argument('<calldata>', 'call data')
        .addOption(new Option('--file <file>', 'file to write Axelar proposal JSON to'))
        .addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarServiceGovernance'))
        .action((target, calldata, options, cmd) => {
            main(cmd.name(), [target, calldata], options);
        });

    program
        .command('submit')
        .description('Submit a scheduled proposal via cross-chain message')
        .argument('<action>', 'governance action (raw, upgrade, transferGovernance, transferOperatorship, withdraw)')
        .argument('<commandId>', 'command id')
        .argument(
            '<activationTime>',
            'proposal activation time as UTC timestamp (YYYY-MM-DDTHH:mm:ss) or relative delay in seconds (numeric)',
        )
        .addOption(
            new Option('--targetContractName <targetContractName>', 'target contract name (required for upgrade, transferGovernance)'),
        )
        .addOption(new Option('--target <target>', 'governance execution target (required for raw action)'))
        .addOption(new Option('--calldata <calldata>', 'calldata (required for raw action)'))
        .addOption(
            new Option('-c, --contractName <contractName>', 'contract name')
                .choices(['InterchainGovernance', 'AxelarServiceGovernance'])
                .default('AxelarServiceGovernance'),
        )
        .addOption(new Option('--newGovernance <governance>', 'governance address').env('GOVERNANCE'))
        .addOption(new Option('--newMintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'))
        .addOption(new Option('--newOperator <newOperator>', 'operator address').env('OPERATOR'))
        .addOption(new Option('--implementation <implementation>', 'new gateway implementation'))
        .addOption(new Option('--amount <amount>', 'withdraw amount'))
        .action((governanceAction, commandId, activationTime, options, cmd) => {
            main(cmd.name(), [governanceAction, commandId, activationTime], options);
        });

    program
        .command('submit-operator')
        .description('Submit an operator proposal via cross-chain message (AxelarServiceGovernance only)')
        .argument('<target>', 'target address')
        .argument('<calldata>', 'call data')
        .argument('<commandId>', 'command id')
        .argument(
            '<activationTime>',
            'proposal activation time as UTC timestamp (YYYY-MM-DDTHH:mm:ss) or relative delay in seconds (numeric)',
        )
        .addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarServiceGovernance'))
        .action((target, calldata, commandId, activationTime, options, cmd) => {
            main(cmd.name(), [target, calldata, commandId, activationTime], options);
        });

    program
        .command('execute-operator-proposal')
        .description('Execute an approved operator proposal (AxelarServiceGovernance only)')
        .argument('<target>', 'target address')
        .argument('<calldata>', 'call data')
        .addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarServiceGovernance'))
        .addOption(new Option('--nativeValue <nativeValue>', 'native value').default('0'))
        .action((target, calldata, options, cmd) => {
            main(cmd.name(), [target, calldata], options);
        });

    program
        .command('is-operator-approved')
        .description('Check whether an operator proposal has been approved (AxelarServiceGovernance only)')
        .argument('<target>', 'target address')
        .argument('<calldata>', 'call data')
        .addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarServiceGovernance'))
        .addOption(new Option('--nativeValue <nativeValue>', 'native value').default('0'))
        .action((target, calldata, options, cmd) => {
            main(cmd.name(), [target, calldata], options);
        });

    addOptionsToCommands(program, addBaseOptions, { address: true });
    addOptionsToCommands(program, addGovernanceOptions, {});
    program.parse();
}

module.exports = {
    governance: main,
    processCommand,
    getProposalCalldata,
    encodeGovernanceProposal,
    getProposalHash,
    getSetupParams,
    ProposalType,
    submitProposalToAxelar,
};
