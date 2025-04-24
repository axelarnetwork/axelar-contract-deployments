'use strict';

const { Contract, nativeToScVal, scValToNative, Address } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const { getWallet, broadcast, addBaseOptions } = require('./utils');
const { loadConfig, printInfo, prompt, validateParameters, saveConfig, addOptionsToCommands, getChainConfig } = require('../common');

const ProposalType = {
    ScheduleTimelock: 1,
    CancelTimelock: 2,
    ApproveOperator: 3,
    CancelOperator: 4,
};

function createProposalPayload(commandType, target, callData, functionName, nativeValue, eta) {
    return nativeToScVal([
        nativeToScVal(commandType, { type: 'u32' }),
        Address.fromString(target).toScVal(),
        nativeToScVal(Buffer.from(callData, 'hex'), { type: 'bytes' }),
        nativeToScVal(functionName, { type: 'symbol' }),
        nativeToScVal(nativeValue, { type: 'i128' }),
        nativeToScVal(eta, { type: 'u64' })
    ]);
}

async function execute(wallet, _, chain, contract, args, options) {
    const [sourceChain, sourceAddress, payload] = args;
    
    validateParameters({
        isNonEmptyString: { sourceChain, sourceAddress },
    });

    const { target, callData, functionName, nativeValue, eta } = JSON.parse(payload);

    validateParameters({
        isValidStellarAddress: { target },
        isNonEmptyString: { function: functionName },
    });

    const proposalPayload = createProposalPayload(
        ProposalType.ScheduleTimelock,
        target,
        callData,
        functionName,
        nativeValue,
        eta || 0
    );

    const operation = contract.call('execute', 
        nativeToScVal(sourceChain, { type: 'string' }),
        nativeToScVal(sourceAddress, { type: 'string' }),
        proposalPayload
    );
    const result = await broadcast(operation, wallet, chain, 'Governance execute', options);
    
    return result;
}

async function executeProposal(wallet, _, chain, contract, args, options) {
    const [target, callData, functionName, nativeValue, tokenAddress] = args;
    
    validateParameters({
        isValidStellarAddress: { target, tokenAddress },
        isNonEmptyString: { function: functionName },
    });

    const operation = contract.call('execute_proposal', 
        Address.fromString(target).toScVal(),
        nativeToScVal(callData, { type: 'bytes' }),
        nativeToScVal(functionName, { type: 'symbol' }),
        nativeToScVal(nativeValue, { type: 'i128' }),
        Address.fromString(tokenAddress).toScVal()
    );
    const result = await broadcast(operation, wallet, chain, 'Execute proposal', options);
    
    return result;
}

async function executeOperatorProposal(wallet, _, chain, contract, args, options) {
    const [target, callData, functionName, nativeValue, tokenAddress] = args;
    
    validateParameters({
        isValidStellarAddress: { target, tokenAddress },
        isNonEmptyString: { function: functionName },
    });

    const operation = contract.call('execute_operator_proposal', 
        Address.fromString(target).toScVal(),
        nativeToScVal(callData, { type: 'bytes' }),
        nativeToScVal(functionName, { type: 'symbol' }),
        nativeToScVal(nativeValue, { type: 'i128' }),
        Address.fromString(tokenAddress).toScVal()
    );
    const result = await broadcast(operation, wallet, chain, 'Execute operator proposal', options);
    
    return result;
}

async function proposalEta(wallet, _, chain, contract, args, options) {
    const [target, callData, functionName, nativeValue] = args;
    
    validateParameters({
        isValidStellarAddress: { target },
        isNonEmptyString: { function: functionName },
    });

    const operation = contract.call('proposal_eta', 
        Address.fromString(target).toScVal(),
        nativeToScVal(callData, { type: 'bytes' }),
        nativeToScVal(functionName, { type: 'symbol' }),
        nativeToScVal(nativeValue, { type: 'i128' })
    );
    const result = await broadcast(operation, wallet, chain, 'Get proposal ETA', options);
    
    return scValToNative(result);
}

async function isOperatorProposalApproved(wallet, _, chain, contract, args, options) {
    const [target, callData, functionName, nativeValue] = args;
    
    validateParameters({
        isValidStellarAddress: { target },
        isNonEmptyString: { function: functionName },
    });

    const operation = contract.call('is_operator_proposal_approved', 
        Address.fromString(target).toScVal(),
        nativeToScVal(Buffer.from(callData, 'hex'), { type: 'bytes' }),
        nativeToScVal(functionName, { type: 'symbol' }),
        nativeToScVal(nativeValue, { type: 'i128' })
    );
    const result = await broadcast(operation, wallet, chain, 'Check operator proposal approval', options);
    
    return scValToNative(result);
}

async function transferOperatorship(wallet, _, chain, contract, args, options) {
    const [newOperator] = args;
    
    validateParameters({
        isValidStellarAddress: { newOperator },
    });

    const operation = contract.call('transfer_operatorship_wrapper', 
        Address.fromString(newOperator).toScVal()
    );
    const result = await broadcast(operation, wallet, chain, 'Transfer operatorship', options);
    
    return result;
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

    const contractAddress = chain.contracts?.AxelarGovernance?.address;

    console.log(contractAddress);

    validateParameters({
        isValidStellarAddress: { contractAddress }
    });

    const contract = new Contract(contractAddress);

    await processor(wallet, config, chain, contract, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('governance').description('Governance contract management');

    program
        .command('execute <sourceChain> <sourceAddress> <payload>')
        .description('Execute a governance command')
        .action((sourceChain, sourceAddress, payload, options) => {
            mainProcessor(execute, [sourceChain, sourceAddress, payload], options);
        });

    program
        .command('execute-proposal <target> <callData> <function> <nativeValue> <tokenAddress>')
        .description('Execute a time-locked proposal')
        .action((target, callData, functionName, nativeValue, tokenAddress, options) => {
            mainProcessor(executeProposal, [target, callData, functionName, nativeValue, tokenAddress], options);
        });

    program
        .command('execute-operator-proposal <target> <callData> <function> <nativeValue> <tokenAddress>')
        .description('Execute an operator-approved proposal')
        .action((target, callData, functionName, nativeValue, tokenAddress, options) => {
            mainProcessor(executeOperatorProposal, [target, callData, functionName, nativeValue, tokenAddress], options);
        });

    program
        .command('proposal-eta <target> <callData> <function> <nativeValue>')
        .description('Get the ETA of a proposal')
        .action((target, callData, functionName, nativeValue, options) => {
            mainProcessor(proposalEta, [target, callData, functionName, nativeValue], options);
        });

    program
        .command('is-operator-proposal-approved <target> <callData> <function> <nativeValue>')
        .description('Check if an operator proposal is approved')
        .action((target, callData, functionName, nativeValue, options) => {
            mainProcessor(isOperatorProposalApproved, [target, callData, functionName, nativeValue], options);
        });

    program
        .command('transfer-operatorship <newOperator>')
        .description('Transfer the operatorship to a new address')
        .action((newOperator, options) => {
            mainProcessor(transferOperatorship, [newOperator], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}

