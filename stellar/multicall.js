'use strict';

const { Contract } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const { getWallet, broadcast, addBaseOptions } = require('./utils');
const { loadConfig, printInfo, prompt, validateParameters, saveConfig, addOptionsToCommands, getChainConfig } = require('../common');
const { functionCallsToScVal } = require('./type-utils');

const validateFunctionCalls = (functionCalls) => {
    if (!Array.isArray(functionCalls) || functionCalls.length === 0) {
        throw new Error('Function calls must be a non-empty array');
    }

    functionCalls.forEach(({ contract, approver, function: fn }) =>
        validateParameters({
            isValidStellarAddress: { contract, approver },
            isNonEmptyString: { function: fn }
        })
    );
};

async function multicall(wallet, _, chain, contract, args, options) {
    const [functionCallsJson] = args;
    const functionCalls = JSON.parse(functionCallsJson);

    validateFunctionCalls(functionCalls);

    const operation = contract.call('multicall', functionCallsToScVal(functionCalls));
    const result = await broadcast(operation, wallet, chain, 'Multicall executed', options);

    printInfo('Multicall results:');
    result.value().forEach((result, i) => printInfo(`Result ${i + 1}:`, result._value ?? 'Call executed successfully'));

    return result.value();
}

async function mainProcessor(processor, args, options) {
    const { yes, env, chainName } = options;
    const config = loadConfig(env);
    const chain = getChainConfig(config, chainName);
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

    saveConfig(config, env);
}

if (require.main === module) {
    const command = new Command();

    command.name('multicall').description('Multicall contract management');

    command
        .command('multicall <functionCallsJson>')
        .description('Execute multiple function calls in a single transaction. Provide function calls as JSON array')
        .action((functionCallsJson, options) => {
            mainProcessor(multicall, [functionCallsJson], options);
        });

    addOptionsToCommands(command, addBaseOptions);

    command.parse();
}
