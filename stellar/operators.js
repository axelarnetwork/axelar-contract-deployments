const { Contract, Address, nativeToScVal } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { getWallet, prepareTransaction, buildTransaction, sendTransaction, estimateCost } = require('./utils');
const { loadConfig, printInfo, parseArgs } = require('../evm/utils');
require('./cli-utils');

async function processCommand(options, _, chain) {
    const [wallet, server] = await getWallet(chain, options);

    const contract = new Contract(options.address || chain.contracts?.axelar_operators?.address);

    let operator, operation;

    if (['is_operator', 'add_operator', 'remove_operator'].includes(options.action)) {
        if (!options.args) throw new Error(`Missing --args operatorAddress the params.`);
        operator = Address.fromString(options.args).toScVal();
    } else {
        operator = Address.fromString(wallet.publicKey()).toScVal();
    }

    switch (options.action) {
        case 'is_operator': {
            operation = contract.call('is_operator', operator);
            break;
        }

        case 'add_operator': {
            operation = contract.call('add_operator', operator);
            break;
        }

        case 'remove_operator': {
            operation = contract.call('remove_operator', operator);
            break;
        }

        case 'refund': {
            // eslint-disable-next-line no-case-declarations
            const gasService = options.target || chain.contracts?.GasService?.address;
            if (!gasService) throw new Error(`Missing AxelarGasService address in the chain info.`);

            const target = Address.fromString(gasService).toScVal();
            const method = nativeToScVal('refund', { type: 'symbol' });
            const [messageId, receiver, tokenAddress, tokenAmount] = parseArgs(options.args || '');
            const args = nativeToScVal([
                messageId,
                Address.fromString(receiver),
                { address: Address.fromString(tokenAddress), amount: tokenAmount },
            ]);

            operation = contract.call('execute', operator, target, method, args);
            break;
        }

        case 'execute': {
            if (!options.target) throw new Error(`Missing target address param.`);
            const target = Address.fromString(options.target).toScVal();

            if (!options.method) throw new Error(`Missing method name param.`);
            const method = nativeToScVal(options.method, { type: 'symbol' });

            const args = nativeToScVal(parseArgs(options.args || ''));

            operation = contract.call('execute', operator, target, method, args);
            break;
        }

        default: {
            throw new Error(`Unknown action: ${options.action}`);
        }
    }

    if (options.estimateCost) {
        const tx = await buildTransaction(operation, server, wallet, chain.networkType, options);
        const resourceCost = await estimateCost(tx, server);
        printInfo('Resource cost', JSON.stringify(resourceCost, null, 2));
        return;
    }

    const signedTx = await prepareTransaction(operation, server, wallet, chain.networkType, options);
    const returnValue = await sendTransaction(signedTx, server);

    if (returnValue) {
        printInfo('Return value', returnValue);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('operators').description('Operators contract management');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );

    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-v, --verbose', 'verbose output').default(false));
    program.addOption(
        new Option('--action <action>', 'operator contract action')
            .choices(['is_operator', 'add_operator', 'remove_operator', 'execute'])
            .makeOptionMandatory(true),
    );
    program.addOption(new Option('--estimateCost', 'estimate on-chain resources').default(false));
    program.addOption(new Option('--address <address>', 'operators contract address'));
    program.addOption(new Option('--args <args>', 'arguments for the contract call'));
    program.addOption(new Option('--target <target>', 'target contract for the execute call'));
    program.addOption(new Option('--method <method>', 'target method for the execute call'));

    program.action((options) => {
        const config = loadConfig(options.env);
        processCommand(options, config, config.stellar);
    });

    program.parse();
}
