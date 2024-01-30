'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { defaultAbiCoder, keccak256, Interface, parseEther },
    Contract,
    BigNumber,
    constants: { AddressZero },
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    getGasOptions,
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
    isValidCalldata,
    writeJSON,
    isKeccak256Hash,
} = require('./utils.js');
const { addBaseOptions } = require('./cli-utils.js');
const { getWallet } = require('./sign-utils.js');
const IAxelarServiceGovernance = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarServiceGovernance.json');
const IGateway = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGateway.json');
const IUpgradable = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IUpgradable.json');
const ProposalType = {
    ScheduleTimelock: 0,
    CancelTimelock: 1,
    ApproveMultisig: 2,
    CancelMultisig: 3,
};

/**
 * Array of proposals across multiple chains. Convenient for constructing Axelar governance proposal file.
 */
let proposals = [];

async function getSetupParams(governance, targetContractName, target, contracts, wallet, options) {
    let setupParams = '0x';

    switch (targetContractName) {
        case 'AxelarGateway': {
            const gateway = new Contract(target, IGateway.abi, wallet);
            const currGovernance = await gateway.governance();
            const currMintLimiter = await gateway.mintLimiter();

            if (currGovernance !== governance.address) {
                printWarn(`Gateway governor ${currGovernance} does not match governance contract: ${governance.address}`);
            }

            let newGovernance = options.newGovernance || contracts.InterchainGovernance?.address;

            if (newGovernance === currGovernance) {
                newGovernance = AddressZero;
            }

            let newMintLimiter = options.newMintLimiter || contracts.Multisig?.address;

            if (newMintLimiter === `${currMintLimiter}`) {
                newMintLimiter = AddressZero;
            }

            if (newGovernance !== '0x' || newMintLimiter !== '0x') {
                setupParams = defaultAbiCoder.encode(['address', 'address', 'bytes'], [newGovernance, newMintLimiter, '0x']);
            }

            break;
        }

        // eslint-disable-next-line no-fallthrough
        case 'InterchainTokenService':

        // eslint-disable-next-line no-fallthrough
        case 'InterchainTokenFactory': {
            // upgrades aren't needed to override any setup params as they can changed via direct methods
            break;
        }
    }

    return setupParams;
}

async function getProposalCalldata(governance, chain, wallet, options) {
    const { action } = options;

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

            if (!isValidAddress(implementation)) {
                throw new Error(`Invalid new implementation address: ${implementation}\nDo you need to pass in 'targetContractName'?`);
            }

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

            title = `Chain ${chain.name} ${options.action} proposal`;
            description = `This proposal ${options.action}s the contract ${target} on chain ${chain.name} to a new implementation contract ${implementation}`;

            break;
        }

        case 'transferGovernance': {
            const newGovernance = options.newGovernance || chain.contracts.InterchainGovernance?.address;

            if (!isValidAddress(newGovernance)) {
                throw new Error(`Invalid new gateway governance address: ${newGovernance}`);
            }

            const gateway = new Contract(target, IGateway.abi, wallet);
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
            if (!isValidDecimal(options.amount)) {
                throw new Error(`Invalid withdraw amount: ${options.amount}`);
            }

            const amount = parseEther(options.amount);
            calldata = governance.interface.encodeFunctionData('withdraw', [options.target, amount]);
            target = governance.address;

            break;
        }

        default: {
            throw new Error(`Unknown governance action ${action}`);
        }
    }

    if (!isValidAddress(target)) {
        throw new Error(`Target address required for this governance action: ${action}`);
    }

    if (!isValidCalldata(calldata)) {
        throw new Error(`Calldata required for this governance action: ${action}`);
    }

    return { target, calldata, title, description };
}

function getGovernanceProposal(commandType, target, calldata, nativeValue, eta) {
    const types = ['uint256', 'address', 'bytes', 'uint256', 'uint256'];
    const values = [commandType, target, calldata, nativeValue, eta];

    return defaultAbiCoder.encode(types, values);
}

async function processCommand(_, chain, options) {
    const { contractName, address, action, proposalAction, date, privateKey, yes } = options;

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

    let nativeValue = options.nativeValue;

    if (!isValidDecimal(nativeValue)) {
        throw new Error(`Invalid native value: ${nativeValue}`);
    }

    const provider = getDefaultProvider(chain.rpc);

    const wallet = await getWallet(privateKey, provider, options);
    await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', governanceAddress);

    const governance = new Contract(governanceAddress, IAxelarServiceGovernance.abi, wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('Proposal Action', action);

    let { target, calldata, title, description } = await getProposalCalldata(governance, chain, wallet, options);
    let commandType = -1;
    let eta = 0;

    // Axelar governance proposal data
    let gmpPayload;

    switch (proposalAction) {
        case 'eta': {
            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            eta = await governance.getTimeLock(proposalHash);

            if (eta.eq(0)) {
                printWarn('Proposal does not exist.');
            }

            printInfo('Proposal ETA', etaToDate(eta));

            return;
        }

        case 'schedule': {
            if (!isValidTimeFormat(date)) {
                throw new Error(`Invalid ETA: ${date}. Please pass the eta in the format YYYY-MM-DDTHH:mm:ss`);
            }

            eta = dateToEta(date);

            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            const minEta = currTime + (await governance.minimumTimeLockDelay()).toNumber();
            printInfo('Minimum eta', etaToDate(minEta));

            if (eta < minEta) {
                printWarn(`${date} is less than the minimum eta.`);
            }

            printInfo('Time difference between current time and eta', etaToDate(eta - currTime));

            const existingProposalEta = await governance.getProposalEta(target, calldata, nativeValue);

            if (!existingProposalEta.eq(BigNumber.from(0))) {
                throw new Error(`Proposal already exists with eta: ${existingProposalEta}.`);
            }

            commandType = ProposalType.ScheduleTimelock;

            break;
        }

        case 'cancel': {
            const currTime = getCurrentTimeInSeconds();
            printInfo('Current time', etaToDate(currTime));

            eta = await governance.geteta(target, calldata, nativeValue);
            printInfo('Proposal eta', etaToDate(eta));

            if (eta.eq(BigNumber.from(0))) {
                printWarn(`Proposal does not exist.`);
            }

            if (eta <= currTime) {
                printWarn(`Proposal eta has already passed.`);
            }

            commandType = ProposalType.CancelTimelock;
            gmpPayload = getGovernanceProposal(ProposalType.CancelTimelock, target, calldata, nativeValue, eta);

            break;
        }

        case 'scheduleMultisig': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${action}`);
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            commandType = ProposalType.ApproveMultisig;

            break;
        }

        case 'cancelMultisig': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${action}`);
            }

            if (!calldata) {
                throw new Error(`Calldata required for this governance action: ${action}`);
            }

            commandType = ProposalType.CancelMultisig;

            break;
        }

        // eslint-disable-next-line no-fallthrough
        case 'submit':

        // eslint-disable-next-line no-fallthrough
        case 'submitMultisig': {
            if (!isKeccak256Hash(options.commandId)) {
                throw new Error(`Invalid command id: ${options.commandId}`);
            }

            eta = dateToEta(date);
            const commandType = proposalAction === 'submit' ? ProposalType.ScheduleTimelock : ProposalType.ApproveMultisig;

            gmpPayload = getGovernanceProposal(commandType, target, calldata, nativeValue, eta);

            if (prompt('Proceed with submitting this proposal?', yes)) {
                throw new Error('Proposal submission cancelled.');
            }

            const tx = await governance.execute(
                options.commandId,
                contracts.InterchainGovernance.governanceChain,
                contracts.InterchainGovernance.governanceAddress,
                gmpPayload,
                gasOptions,
            );

            printInfo('Proposal submitted', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, governance, 'ProposalScheduled');

            if (!eventEmitted) {
                throw new Error('Proposal submission failed.');
            }

            return;
        }

        case 'execute': {
            if (options.proposal) {
                printInfo('Decoding proposal to get governance data');

                [_, target, calldata, nativeValue, _] = defaultAbiCoder.decode(
                    ['uint256', 'address', 'bytes', 'uint256', 'uint256'],
                    options.proposal,
                );

                if (!isValidCalldata(calldata)) {
                    throw new Error(`Calldata required for this governance action: ${action}`);
                }
            }

            const proposalHash = keccak256(defaultAbiCoder.encode(['address', 'bytes', 'uint256'], [target, calldata, nativeValue]));
            eta = await governance.getTimeLock(proposalHash);

            if (eta.eq(0)) {
                throw new Error('Proposal does not exist.');
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

        case 'executeMultisig': {
            if (contractName === 'InterchainGovernance') {
                throw new Error(`Invalid governance action for InterchainGovernance: ${action}`);
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

            if (prompt('Proceed with executing this proposal?', yes)) {
                throw new Error('Proposal execution cancelled.');
            }

            try {
                const tx = await governance.executeMultisigProposal(target, calldata, nativeValue, gasOptions);
                receipt = await tx.wait(chain.confirmations);
            } catch (error) {
                printError(error);
                return;
            }

            const eventEmitted = wasEventEmitted(receipt, governance, 'MultisigExecuted');

            if (!eventEmitted) {
                throw new Error('Multisig proposal execution failed.');
            }

            printInfo('Multisig proposal executed.');

            break;
        }

        default: {
            throw new Error(`Unknown proposal action ${proposalAction}`);
        }
    }

    if (commandType !== -1) {
        gmpPayload = getGovernanceProposal(commandType, target, calldata, nativeValue, eta);

        const payloadBase64 = Buffer.from(`${gmpPayload}`.slice(2), 'hex').toString('base64');

        printInfo('Destination chain', chain.name);
        printInfo('Destination governance contract', governanceAddress);
        printInfo('Axelar Proposal call contract payload', gmpPayload);
        printInfo('Axelar Proposal payload hash', keccak256(gmpPayload));
        printInfo('Governance target', target);
        printInfo('Governance data', calldata);
        printInfo('Governance native value', nativeValue || '0');
        printInfo('Date', date);

        const proposal = {
            title,
            description,
            contract_calls: [
                {
                    chain: chain.axelarId,
                    contract_address: governanceAddress,
                    payload: payloadBase64,
                },
            ],
        };

        // Print all proposals together
        proposals.push(proposal.contract_calls[0]);

        // printInfo('Proposal', JSON.stringify(proposal, null, 2));
        // console.log(JSON.stringify(proposal.contract_calls[0]));
    }
}

async function main(options) {
    proposals = [];

    await mainProcessor(options, processCommand);

    const proposal = {
        title: 'Interchain Governance Proposal',
        description: 'Interchain Governance Proposal',
        contract_calls: proposals,
    };

    if (proposals.length > 0) {
        const proposalJSON = JSON.stringify(proposal, null, 2);

        if (options.file) {
            writeJSON(proposal, options.file);
            printInfo('Proposal written to file', options.file);
        } else {
            printInfo('Proposal', proposalJSON);
        }
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('governance').description('Script to manage interchain governance actions');

    addBaseOptions(program, { address: true });

    program.addOption(
        new Option('-c, --contractName <contractName>', 'contract name')
            .choices(['InterchainGovernance', 'AxelarServiceGovernance'])
            .default('InterchainGovernance'),
    );
    program.addOption(new Option('--targetContractName <targetContractName>', 'target contract name'));
    program.addOption(new Option('--action <action>', 'governance action').choices(['raw', 'upgrade', 'transferGovernance', 'withdraw']));
    program.addOption(
        new Option('--proposalAction <proposalAction>', 'governance proposal action').choices([
            'eta',
            'schedule',
            'submit',
            'execute',
            'cancel',
            'scheduleMultisig',
            'submitMultisig',
            'executeMultisig',
            'cancelMultisig',
        ]),
    );
    program.addOption(new Option('--date <date>', 'proposal activation date'));
    program.addOption(new Option('--file <file>', 'file to write Axelar proposal JSON to'));
    program.addOption(new Option('--newGovernance <governance>', 'governance address').env('GOVERNANCE'));
    program.addOption(new Option('--newMintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'));
    program.addOption(new Option('--commandId <commandId>', 'command id'));
    program.addOption(new Option('--target <target>', 'governance execution target'));
    program.addOption(new Option('--calldata <calldata>', 'calldata'));
    program.addOption(new Option('--nativeValue <nativeValue>', 'nativeValue').default(0));
    program.addOption(new Option('--proposal <proposal>', 'governance proposal payload'));
    program.addOption(new Option('--amount <amount>', 'withdraw amount'));
    program.addOption(new Option('--implementation <implementation>', 'new gateway implementation'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
