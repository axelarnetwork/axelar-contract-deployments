'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { defaultAbiCoder, keccak256, parseEther },
    Contract,
    BigNumber,
    constants: { AddressZero },
} = ethers;
const { Command, Option, Argument } = require('commander');
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
    isConsensusChain,
    isEvmChain,
    isValidAddress,
} = require('./utils.js');
const { addBaseOptions, addOptionsToCommands } = require('./cli-utils');
const { getWallet } = require('./sign-utils.js');
const { submitCallContracts, payloadToHexBinary, GOVERNANCE_MODULE_ADDRESS } = require('../cosmwasm/utils');
const { mainProcessor: cosmwasmMainProcessor } = require('../cosmwasm/processor');
const { executeByGovernance } = require('../cosmwasm/proposal-utils');
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
    program.addOption(new Option('--generate-only <file>', 'generate Axelar proposal JSON to the given file instead of submitting'));
    program.addOption(
        new Option('--standardProposal', 'submit as a standard proposal instead of expedited (default is expedited)').default(false),
    );

    return program;
}

function ensureAxelarServiceGovernance(contractName, action) {
    if (contractName === 'InterchainGovernance') {
        throw new Error(`Invalid governance action for InterchainGovernance: ${action}`);
    }
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
    const contractName = options.contractName;
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
            ensureAxelarServiceGovernance(contractName, 'transferOperatorship');

            const newOperator = options.newOperator;

            validateParameters({
                isValidAddress: { newOperator },
            });

            calldata = governance.interface.encodeFunctionData('transferOperatorship', [newOperator]);
            title = `Chain ${chain.name} transfer operatorship`;
            description = `Transfers operatorship of AxelarServiceGovernance to ${newOperator} on chain ${chain.name}`;
            target = governance.address;

            break;
        }

        case 'withdraw': {
            validateParameters({
                isValidDecimal: { amount: options.amount },
                isValidAddress: { target: options.target },
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

function decodeProposalPayload(proposal) {
    const decoded = defaultAbiCoder.decode(['uint256', 'address', 'bytes', 'uint256', 'uint256'], proposal);
    return {
        commandType: decoded[0],
        target: decoded[1],
        calldata: decoded[2],
        nativeValue: decoded[3].toString(),
        eta: decoded[4],
    };
}

function ensureNonZeroActivationTime(commandName, activationTime) {
    if (String(activationTime).trim() === '0') {
        throw new Error(
            `${commandName} does not support activationTime=0. Use an explicit UTC timestamp (YYYY-MM-DDTHH:mm:ss). `
        );
    }
}

async function processCommand(_axelar, chain, _chains, action, options) {
    if (!isEvmChain(chain)) {
        throw new Error(`Chain "${chain?.name}" is not an EVM chain (chainType must be "evm")`);
    }
    const { contractName, address, privateKey, args = [] } = options;

    const governanceAddress = getGovernanceAddress(chain, contractName, address);
    const provider = getDefaultProvider(chain.rpc);
    const wallet = await getWallet(privateKey, provider, options);
    await printWalletInfo(wallet, options);

    printInfo('Governance Contract name', contractName);
    printInfo('Governance Contract address', governanceAddress);

    const governance = new Contract(governanceAddress, IAxelarServiceGovernance.abi, wallet);
    const gasOptions = await getGasOptions(chain, options, contractName);

    let nativeValue = options.nativeValue;
    validateParameters({
        isValidDecimal: { nativeValue },
    });
    nativeValue = nativeValue.toString();
    printInfo('Native value', nativeValue);

    switch (action) {
        case 'eta': {
            let target = options.target;
            let calldata = options.calldata;

            if (options.proposal) {
                const decoded = decodeProposalPayload(options.proposal);
                target = decoded.target;
                calldata = decoded.calldata;
                nativeValue = decoded.nativeValue;
            }

            validateParameters({
                isValidAddress: { target },
                isValidCalldata: { calldata },
            });

            const proposalEta = await governance.getProposalEta(target, calldata, nativeValue);

            if (proposalEta.eq(0)) {
                printWarn('Proposal does not exist.');
            } else {
                printInfo('Proposal ETA', etaToDate(proposalEta));
            }

            return null;
        }

        case 'schedule': {
            const [action, activationTime] = args;

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
            } else {
                printInfo('Time difference between current time and eta', etaToDate(eta - currTime));
            }

            const { target, calldata } = await getProposalCalldata(governance, chain, wallet, action, options);
            printInfo('Governance target (for eta/execute)', target);
            printInfo('Governance calldata (for eta/execute)', calldata);

            const existingProposalEta = await governance.getProposalEta(target, calldata, nativeValue);
            if (!existingProposalEta.eq(BigNumber.from(0))) {
                throw new Error(`Proposal already exists with eta: ${existingProposalEta}.`);
            }

            const gmpPayload = encodeGovernanceProposal(ProposalType.ScheduleTimelock, target, calldata, nativeValue, eta);
            printInfo('Governance proposal payload (for eta/execute)', gmpPayload);

            return createGMPProposalJSON(chain, governanceAddress, gmpPayload);
        }

        case 'cancel': {
            const [action, activationTime] = args;

            validateParameters({
                isValidTimeFormat: { activationTime },
            });
            ensureNonZeroActivationTime('cancel', activationTime);

            const { target, calldata } = await getProposalCalldata(governance, chain, wallet, action, options);
            printInfo('Governance target (for execute)', target);
            printInfo('Governance calldata (for execute)', calldata);

            const eta = dateToEta(activationTime);
            printInfo('Proposal eta', etaToDate(eta));

            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            const existingProposalEta = await governance.getProposalEta(target, calldata, nativeValue);
            printInfo('Proposal eta', etaToDate(existingProposalEta));

            if (existingProposalEta.eq(BigNumber.from(0))) {
                printWarn('Proposal does not exist.');
            }

            if (existingProposalEta.toNumber() <= currTime) {
                printWarn('Proposal eta has already passed.');
            }

            const gmpPayload = encodeGovernanceProposal(ProposalType.CancelTimelock, target, calldata, nativeValue, eta);
            printInfo('Governance proposal payload (for execute)', gmpPayload);

            return createGMPProposalJSON(chain, governanceAddress, gmpPayload);
        }

        case 'schedule-operator': {
            ensureAxelarServiceGovernance(contractName, 'scheduleOperator');

            const [action, activationTime] = args;

            validateParameters({
                isValidTimeFormat: { activationTime },
            });

            const { target, calldata } = await getProposalCalldata(governance, chain, wallet, action, options);
            printInfo('Governance target (for execute-operator-proposal)', target);
            printInfo('Governance calldata (for execute-operator-proposal)', calldata);

            const isApproved = await governance.isOperatorProposalApproved(target, calldata, nativeValue);
            if (isApproved) {
                throw new Error('Operator proposal is already approved.');
            }

            const eta = dateToEta(activationTime);
            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            const minEta = currTime + (await governance.minimumTimeLockDelay()).toNumber();
            printInfo('Minimum eta', etaToDate(minEta));

            if (eta < minEta) {
                printWarn(`${activationTime} is less than the minimum eta.`);
            } else {
                printInfo('Time difference between current time and eta', etaToDate(eta - currTime));
            }

            const gmpPayload = encodeGovernanceProposal(ProposalType.ApproveOperator, target, calldata, nativeValue, eta);
            printInfo('Governance proposal payload (for execute-operator-proposal)', gmpPayload);

            return createGMPProposalJSON(chain, governanceAddress, gmpPayload);
        }

        case 'cancel-operator': {
            ensureAxelarServiceGovernance(contractName, 'cancelOperator');

            const [action, activationTime] = args;

            validateParameters({
                isValidTimeFormat: { activationTime },
            });
            ensureNonZeroActivationTime('cancel-operator', activationTime);

            const { target, calldata } = await getProposalCalldata(governance, chain, wallet, action, options);

            printInfo('Governance target (for execute-operator-proposal)', target);
            printInfo('Governance calldata (for execute-operator-proposal)', calldata);

            const eta = dateToEta(activationTime);
            printInfo('Proposal eta', etaToDate(eta));
            const isApproved = await governance.isOperatorProposalApproved(target, calldata, nativeValue);

            if (!isApproved) {
                throw new Error('Operator proposal is not approved.');
            }

            const gmpPayload = encodeGovernanceProposal(ProposalType.CancelOperator, target, calldata, nativeValue, eta);
            printInfo('Governance proposal payload (for execute-operator-proposal)', gmpPayload);

            return createGMPProposalJSON(chain, governanceAddress, gmpPayload);
        }

        case 'submit': {
            const [proposaltype, action, commandId, activationTime] = args;

            validateParameters({
                isKeccak256Hash: { commandId },
                isValidTimeFormat: { activationTime },
            });
            ensureNonZeroActivationTime('submit', activationTime);

            printInfo('Proposal type', proposaltype);
            printInfo('Command ID', commandId);

            const eta = dateToEta(activationTime);
            printInfo('Proposal eta', etaToDate(eta));

            const { target, calldata } = await getProposalCalldata(governance, chain, wallet, action, options);
            printInfo('Governance target', target);
            printInfo('Governance calldata', calldata);

            const gmpPayload = encodeGovernanceProposal(
                proposaltype == 'schedule' ? ProposalType.ScheduleTimelock : ProposalType.CancelTimelock,
                target,
                calldata,
                nativeValue,
                eta,
            );

            if (prompt('Proceed with submitting this proposal?', options.yes)) {
                throw new Error('Proposal submission cancelled.');
            }

            const tx = await governance.execute(
                commandId,
                isConsensusChain(chain) ? 'Axelarnet' : 'axelar',
                GOVERNANCE_MODULE_ADDRESS,
                gmpPayload,
                gasOptions,
            );

            await handleTransactionWithEvent(
                tx,
                chain,
                governance,
                'Proposal submission',
                proposaltype == 'schedule' ? 'ProposalScheduled' : 'ProposalCancelled',
            );
            printInfo('Proposal submitted.');
            return null;
        }

        case 'submit-operator': {
            ensureAxelarServiceGovernance(contractName, 'submitOperator');

            const [proposaltype, action, commandId, activationTime] = args;

            validateParameters({
                isKeccak256Hash: { commandId },
                isValidTimeFormat: { activationTime },
            });
            ensureNonZeroActivationTime('submit-operator', activationTime);

            printInfo('Proposal type', proposaltype);
            printInfo('Command ID', commandId);

            const eta = dateToEta(activationTime);
            printInfo('Proposal eta', etaToDate(eta));

            const { target, calldata } = await getProposalCalldata(governance, chain, wallet, action, options);
            printInfo('Governance target', target);
            printInfo('Governance calldata', calldata);

            const gmpPayload = encodeGovernanceProposal(
                proposaltype == 'schedule-operator' ? ProposalType.ApproveOperator : ProposalType.CancelOperator,
                target,
                calldata,
                nativeValue,
                eta,
            );

            if (prompt('Proceed with submitting this proposal?', options.yes)) {
                throw new Error('Proposal submission cancelled.');
            }

            const tx = await governance.execute(
                commandId,
                isConsensusChain(chain) ? 'Axelarnet' : 'axelar',
                GOVERNANCE_MODULE_ADDRESS,
                gmpPayload,
                gasOptions,
            );

            await handleTransactionWithEvent(
                tx,
                chain,
                governance,
                'Proposal submission',
                proposaltype == 'schedule-operator' ? 'OperatorProposalApproved' : 'OperatorProposalCancelled',
            );
            printInfo('Operator proposal submitted.');
            return null;
        }

        case 'execute': {
            let target = options.target;
            let calldata = options.calldata;

            if (options.proposal) {
                printInfo('Decoding proposal to get governance data');
                const decoded = decodeProposalPayload(options.proposal);
                target = decoded.target;
                calldata = decoded.calldata;
                nativeValue = decoded.nativeValue;
            }

            validateParameters({
                isValidAddress: { target },
                isValidCalldata: { calldata },
            });

            const eta = await governance.getProposalEta(target, calldata, nativeValue);

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

            printInfo('nativeValue', nativeValue.toString());

            const tx = await governance.executeProposal(target, calldata, nativeValue, { value: nativeValue, ...gasOptions });
            await handleTransactionWithEvent(tx, chain, governance, 'Proposal execution', 'ProposalExecuted');
            printInfo('Proposal executed.');
            return null;
        }

        case 'execute-operator-proposal': {
            ensureAxelarServiceGovernance(contractName, 'execute-operator-proposal');

            let target = options.target;
            let calldata = options.calldata;

            if (options.proposal) {
                printInfo('Decoding proposal to get governance data');
                const decoded = decodeProposalPayload(options.proposal);
                target = decoded.target;
                calldata = decoded.calldata;
                nativeValue = decoded.nativeValue;
            }

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

            printInfo('nativeValue', nativeValue.toString());

            const tx = await governance.executeOperatorProposal(target, calldata, nativeValue, { value: nativeValue, ...gasOptions });
            await handleTransactionWithEvent(tx, chain, governance, 'Operator proposal execution', 'OperatorProposalExecuted');
            printInfo('Operator proposal executed.');
            return null;
        }

        case 'is-operator-approved': {
            ensureAxelarServiceGovernance(contractName, 'is-operator-approved');

            let target = options.target;
            let calldata = options.calldata;

            if (options.proposal) {
                printInfo('Decoding proposal to get governance data');
                const decoded = decodeProposalPayload(options.proposal);
                target = decoded.target;
                calldata = decoded.calldata;
                nativeValue = decoded.nativeValue;
            }

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
        submitOptions.deposit = config.proposalDepositAmount();
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
    const consensusProposals = [];
    const amplifierAxelarnetMsgs = [];

    await mainProcessor(options, async (axelar, chain, chains, options) => {
        const proposal = await processCommand(axelar, chain, chains, action, options);
        if (proposal) {
            if (isConsensusChain(chain)) {
                consensusProposals.push(proposal);
            } else {
                amplifierAxelarnetMsgs.push({
                    call_contract: {
                        destination_chain: proposal.chain,
                        destination_address: proposal.contract_address,
                        payload: payloadToHexBinary(proposal.payload),
                    },
                });
            }
        }
    });

    const title = 'Interchain Governance Proposal';
    const description = 'Interchain Governance Proposal';

    const hasConsensus = consensusProposals.length > 0;
    const hasAmplifier = amplifierAxelarnetMsgs.length > 0;

    if (hasConsensus) {
        const proposal = { title, description, contract_calls: consensusProposals };
        printInfo('Consensus-chain proposal (CallContractsProposal)', JSON.stringify(proposal, null, 2));

        if (options.generateOnly) {
            writeJSON(proposal, options.generateOnly);
            printInfo('Consensus proposal written to file', options.generateOnly);
        } else if (!prompt('Proceed with submitting this consensus-chain proposal to Axelar?', options.yes)) {
            await submitProposalToAxelar(proposal, options);
        }
    }

    if (hasAmplifier) {
        const amplifierPreview = {
            title,
            description,
            contractName: 'AxelarnetGateway',
            msgs: amplifierAxelarnetMsgs,
        };
        printInfo('Amplifier-chain proposal (AxelarnetGateway.call_contract)', JSON.stringify(amplifierPreview, null, 2));

        if (options.generateOnly) {
            writeJSON(amplifierPreview, options.generateOnly);
            printInfo('Amplifier proposal written to file', options.generateOnly);
        } else if (!prompt('Proceed with submitting this amplifier-chain proposal to Axelar?', options.yes)) {
            const submitFn = async (client, config, submitOptions, _args, fee) => {
                submitOptions.deposit = config.proposalDepositAmount();

                const msgs = amplifierAxelarnetMsgs.map((msg) => JSON.stringify(msg));
                await executeByGovernance(
                    client,
                    config,
                    { ...submitOptions, contractName: 'AxelarnetGateway', msg: msgs, title, description },
                    undefined,
                    fee,
                );
            };

            const submitOptions = {
                env: options.env,
                mnemonic: options.mnemonic,
                contractName: 'AxelarnetGateway',
                title,
                description,
                yes: options.yes,
                rpc: options.rpc,
            };

            await cosmwasmMainProcessor(submitFn, submitOptions);
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
        .argument('<action>', 'governance action (raw, upgrade, transferOperatorship, withdraw)')
        .argument('<activationTime>', 'proposal activation time as UTC timestamp (YYYY-MM-DDTHH:mm:ss)')
        .addOption(new Option('--targetContractName <targetContractName>', 'target contract name (required for upgrade)'))
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
        .action((governanceAction, activationTime, options, cmd) => {
            main(cmd.name(), [governanceAction, activationTime], options);
        });

    program
        .command('cancel')
        .description('Cancel a scheduled timelock proposal')
        .argument('<action>', 'governance action (raw, upgrade, transferOperatorship, withdraw)')
        .argument('<activationTime>', 'proposal activation time as UTC timestamp (YYYY-MM-DDTHH:mm:ss)')
        .addOption(new Option('--targetContractName <targetContractName>', 'target contract name (required for upgrade)'))
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
        .action((governanceAction, activationTime, options, cmd) => {
            main(cmd.name(), [governanceAction, activationTime], options);
        });

    program
        .command('submit')
        .description('Submit a scheduled proposal via cross-chain message')
        .addArgument(new Argument('<proposaltype>', 'proposal type (schedule, cancel)').choices(['schedule', 'cancel']))
        .argument('<action>', 'governance action (raw, upgrade, transferOperatorship, withdraw)')
        .argument('<commandId>', 'command id')
        .argument('<activationTime>', 'proposal activation time as UTC timestamp (YYYY-MM-DDTHH:mm:ss)')
        .addOption(new Option('--targetContractName <targetContractName>', 'target contract name (required for upgrade)'))
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
        .action((proposaltype, governanceAction, commandId, activationTime, options, cmd) => {
            main(cmd.name(), [proposaltype, governanceAction, commandId, activationTime], options);
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
        .argument('<action>', 'governance action (raw, upgrade, transferOperatorship, withdraw)')
        .argument('<activationTime>', 'proposal activation time as UTC timestamp (YYYY-MM-DDTHH:mm:ss)')
        .addOption(new Option('--targetContractName <targetContractName>', 'target contract name (required for upgrade)'))
        .addOption(new Option('--target <target>', 'governance execution target (required for raw action)'))
        .addOption(new Option('--calldata <calldata>', 'calldata (required for raw action)'))
        .addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarServiceGovernance'))
        .addOption(new Option('--newGovernance <governance>', 'governance address').env('GOVERNANCE'))
        .addOption(new Option('--newMintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'))
        .addOption(new Option('--newOperator <newOperator>', 'operator address').env('OPERATOR'))
        .addOption(new Option('--implementation <implementation>', 'new gateway implementation'))
        .addOption(new Option('--amount <amount>', 'withdraw amount'))
        .action((governanceAction, activationTime, options, cmd) => {
            main(cmd.name(), [governanceAction, activationTime], options);
        });

    program
        .command('cancel-operator')
        .description('Cancel an operator proposal (AxelarServiceGovernance only)')
        .argument('<action>', 'governance action (raw, upgrade, transferOperatorship, withdraw)')
        .argument('<activationTime>', 'proposal activation time as UTC timestamp (YYYY-MM-DDTHH:mm:ss)')
        .addOption(new Option('--targetContractName <targetContractName>', 'target contract name (required for upgrade)'))
        .addOption(new Option('--target <target>', 'governance execution target (required for raw action)'))
        .addOption(new Option('--calldata <calldata>', 'calldata (required for raw action)'))
        .addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarServiceGovernance'))
        .addOption(new Option('--newGovernance <governance>', 'governance address').env('GOVERNANCE'))
        .addOption(new Option('--newMintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'))
        .addOption(new Option('--newOperator <newOperator>', 'operator address').env('OPERATOR'))
        .addOption(new Option('--implementation <implementation>', 'new gateway implementation'))
        .addOption(new Option('--amount <amount>', 'withdraw amount'))
        .action((governanceAction, activationTime, options, cmd) => {
            main(cmd.name(), [governanceAction, activationTime], options);
        });

    program
        .command('submit-operator')
        .description('Manually submit an operator proposal GMP message if relayers fail (AxelarServiceGovernance only)')
        .addArgument(
            new Argument('<proposaltype>', 'proposal type (schedule-operator, cancel-operator)').choices([
                'schedule-operator',
                'cancel-operator',
            ]),
        )
        .argument('<action>', 'governance action (raw, upgrade, transferOperatorship, withdraw)')
        .argument('<commandId>', 'command id')
        .argument('<activationTime>', 'proposal activation time as UTC timestamp (YYYY-MM-DDTHH:mm:ss)')

        .addOption(new Option('--targetContractName <targetContractName>', 'target contract name (required for upgrade)'))
        .addOption(new Option('--target <target>', 'governance execution target (required for raw action)'))
        .addOption(new Option('--calldata <calldata>', 'calldata (required for raw action)'))
        .addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarServiceGovernance'))
        .addOption(new Option('--newGovernance <governance>', 'governance address').env('GOVERNANCE'))
        .addOption(new Option('--newMintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'))
        .addOption(new Option('--newOperator <newOperator>', 'operator address').env('OPERATOR'))
        .addOption(new Option('--implementation <implementation>', 'new gateway implementation'))
        .addOption(new Option('--amount <amount>', 'withdraw amount'))
        .action((proposaltype, governanceAction, commandId, activationTime, options, cmd) => {
            main(cmd.name(), [proposaltype, governanceAction, commandId, activationTime], options);
        });

    program
        .command('execute-operator-proposal')
        .description('Execute an approved operator proposal (AxelarServiceGovernance only)')
        .addOption(new Option('--target <target>', 'target address (required if --proposal not provided)'))
        .addOption(new Option('--calldata <calldata>', 'call data (required if --proposal not provided)'))
        .addOption(new Option('--proposal <proposal>', 'governance proposal payload (alternative to target/calldata)'))
        .addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarServiceGovernance'))
        .action((options, cmd) => {
            if (!options.proposal && (!options.target || !options.calldata)) {
                throw new Error('Either --proposal or both --target and --calldata must be provided');
            }
            main(cmd.name(), [], options);
        });

    program
        .command('is-operator-approved')
        .description('Check whether an operator proposal has been approved (AxelarServiceGovernance only)')
        .addOption(new Option('--target <target>', 'target address (required if --proposal not provided)'))
        .addOption(new Option('--calldata <calldata>', 'call data (required if --proposal not provided)'))
        .addOption(new Option('--proposal <proposal>', 'governance proposal payload (alternative to target/calldata)'))
        .addOption(new Option('-c, --contractName <contractName>', 'contract name').default('AxelarServiceGovernance'))
        .action((options, cmd) => {
            if (!options.proposal && (!options.target || !options.calldata)) {
                throw new Error('Either --proposal or both --target and --calldata must be provided');
            }
            main(cmd.name(), [], options);
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
    decodeProposalPayload,
};
