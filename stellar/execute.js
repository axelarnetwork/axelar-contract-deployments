const { Contract, Address, nativeToScVal, scValToNative } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { getWallet, prepareTransaction, buildTransaction, sendTransaction, estimateCost } = require('./utils');
const { loadConfig, printInfo, parseArgs } = require('../evm/utils');
require('./cli-utils');

function parseArgTypes(argTypesString) {
    if (!argTypesString) return null;

    try {
        return JSON.parse(argTypesString);
    } catch (error) {
        throw new Error(`Invalid JSON format for argTypes: ${error.message}`);
    }
}

function convertArg(arg, type) {
    switch (type.type) {
        case 'address':
            return Address.fromString(arg);
        case 'u32':
        case 'i32':
            return parseInt(arg);
        case 'u64':
        case 'i64':
            return BigInt(arg);
        case 'u128':
        case 'i128':
        case 'u256':
        case 'i256':
            return BigInt(arg);
        case 'bytes':
            return Buffer.from(arg, 'hex');
        case 'symbol':
            return arg;
        default:
            return arg;
    }
}

async function processCommand(options, _, chain) {
    const [wallet, server] = await getWallet(chain, options);

    let contractAddress;

    if (options.contractName) {
        contractAddress = chain.contracts?.[options.contractName]?.address;

        if (!contractAddress) {
            throw new Error(`Contract ${options.contractName} not found in chain configuration`);
        }
    } else if (options.contractAddress) {
        contractAddress = options.contractAddress;
    } else {
        throw new Error('Either contractName or contractAddress must be provided');
    }

    const contract = new Contract(contractAddress);

    if (!options.method) {
        throw new Error('Method name is required');
    }

    const argTypes = parseArgTypes(options.argTypes);
    const args = parseArgs(options.args || '');

    if (argTypes && args.length !== argTypes.length) {
        throw new Error('Number of arguments does not match the number of argument types');
    }

    const convertedArgs = argTypes ? args.map((arg, index) => convertArg(arg, argTypes[index])) : args;

    const scvArgs = convertedArgs.map((arg, index) => nativeToScVal(arg, argTypes ? argTypes[index] : undefined));

    const operation = contract.call(options.method, ...scvArgs);

    if (options.estimateCost) {
        const tx = await buildTransaction(operation, server, wallet, chain.networkType, options);
        const resourceCost = await estimateCost(tx, server);
        printInfo('Resource cost', JSON.stringify(resourceCost, null, 2));
        return;
    }

    const signedTx = await prepareTransaction(operation, server, wallet, chain.networkType, options);
    const returnValue = await sendTransaction(signedTx, server);

    if (returnValue) {
        printInfo('Return value', scValToNative(returnValue));
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('execute').description('Generic contract method execution');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );

    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-v, --verbose', 'verbose output').default(false));
    program.addOption(new Option('--estimateCost', 'estimate on-chain resources').default(false));
    program.addOption(new Option('--contractName <contractName>', 'name of the contract in chain configuration'));
    program.addOption(new Option('--contractAddress <contractAddress>', 'address of the contract'));
    program.addOption(new Option('--method <method>', 'method to call on the contract').makeOptionMandatory(true));
    program.addOption(new Option('--args <args>', 'arguments for the contract call'));
    program.addOption(new Option('--argTypes <argTypes>', 'JSON string of argument types'));

    program.action((options) => {
        const config = loadConfig(options.env);
        processCommand(options, config, config.stellar);
    });

    program.parse();
}
