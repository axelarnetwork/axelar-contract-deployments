'use strict';

const { Contract } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const { getWallet, broadcast, addBaseOptions } = require('./utils');
const { loadConfig, printInfo, prompt, validateParameters, saveConfig, addOptionsToCommands, getChainConfig } = require('../common');
const { functionCallsToScVal } = require('./type-utils');

async function multicall(wallet, _, chain, contract, args, options) {
    const [functionCallsJson] = args;
    const functionCalls = JSON.parse(functionCallsJson);

    if (!Array.isArray(functionCalls)) {
        throw new Error('Function calls must be an array');
    }

    // Validate function calls
    functionCalls.forEach((call) => {
        validateParameters({
            isValidStellarAddress: { contract: call.contract, approver: call.approver },
            isNonEmptyString: { function: call.function },
        });
    });

    const scvalFunctionCalls = functionCallsToScVal(functionCalls);
    const operation = contract.call('multicall', scvalFunctionCalls);
    const result = await broadcast(operation, wallet, chain, 'Multicall executed', options);

    printInfo('Multicall results:');
    const results = result.value();
    results.forEach((result, i) => {
        printInfo(`Result ${i + 1}:`, '_value' in result ? result._value : 'Call executed successfully');
    });

    return results;
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

    const contractAddress = chain.contracts?.Multicall?.address;

    validateParameters({
        isValidStellarAddress: { contractAddress },
    });

    const contract = new Contract(contractAddress);

    await processor(wallet, config, chain, contract, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('multicall').description('Multicall contract management');

    program
        .command('multicall <functionCallsJson>')
        .description('Execute multiple function calls in a single transaction. Provide function calls as JSON array')
        .action((functionCallsJson, options) => {
            mainProcessor(multicall, [functionCallsJson], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
