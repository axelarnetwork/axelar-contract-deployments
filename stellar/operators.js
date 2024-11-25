const { Contract, Address, nativeToScVal } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { getWallet, broadcast, addBaseOptions } = require('./utils');
const { loadConfig, printInfo, parseArgs, validateParameters } = require('../evm/utils');
const { getChainConfig } = require('../common');
require('./cli-utils');

async function processCommand(options, _, chain) {
    const wallet = await getWallet(chain, options);

    const contract = new Contract(options.address || chain.contracts?.axelar_operators?.address);

    let operation;

    switch (options.action) {
        case 'is_operator': {
            if (!options.args) {
                throw new Error(`Missing --args operatorAddress the params.`);
            }

            const operator = Address.fromString(options.args).toScVal();
            operation = contract.call('is_operator', operator);
            break;
        }

        case 'add_operator': {
            if (!options.args) {
                throw new Error(`Missing --args operatorAddress the params.`);
            }

            const operator = Address.fromString(options.args).toScVal();
            operation = contract.call('add_operator', operator);
            break;
        }

        case 'remove_operator': {
            if (!options.args) {
                throw new Error(`Missing --args operatorAddress the params.`);
            }

            const operator = Address.fromString(options.args).toScVal();
            operation = contract.call('remove_operator', operator);
            break;
        }

        case 'refund': {
            const operator = Address.fromString(wallet.publicKey()).toScVal();
            const gasService = options.target || chain.contracts?.axelar_gas_service?.address;

            if (!gasService) {
                throw new Error(`Missing AxelarGasService address in the chain info.`);
            }

            const target = Address.fromString(gasService).toScVal();
            const method = nativeToScVal('refund', { type: 'symbol' });
            const [messageId, receiver, tokenAddress, tokenAmount] = parseArgs(options.args || '');

            validateParameters({
                isNonEmptyString: { messageId, receiver, tokenAddress },
                isValidNumber: { tokenAmount },
            });

            const args = nativeToScVal([
                messageId,
                Address.fromString(receiver),
                { address: Address.fromString(tokenAddress), amount: tokenAmount },
            ]);

            operation = contract.call('execute', operator, target, method, args);
            break;
        }

        case 'execute': {
            const operator = Address.fromString(wallet.publicKey()).toScVal();

            if (!options.target) {
                throw new Error(`Missing target address param.`);
            }

            const target = Address.fromString(options.target).toScVal();

            if (!options.method) {
                throw new Error(`Missing method name param.`);
            }

            const method = nativeToScVal(options.method, { type: 'symbol' });

            const args = nativeToScVal(parseArgs(options.args || ''));

            operation = contract.call('execute', operator, target, method, args);
            break;
        }

        default: {
            throw new Error(`Unknown action: ${options.action}`);
        }
    }

    const returnValue = await broadcast(operation, wallet, chain, `${options.action} performed`, options);

    if (returnValue.value()) {
        printInfo('Return value', returnValue.value());
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('operators').description('Operators contract management');

    addBaseOptions(program, { address: true });
    program.addOption(
        new Option('--action <action>', 'operator contract action')
            .choices(['is_operator', 'add_operator', 'remove_operator', 'refund', 'execute'])
            .makeOptionMandatory(true),
    );
    program.addOption(new Option('--args <args>', 'arguments for the contract call'));
    program.addOption(new Option('--target <target>', 'target contract for the execute call'));
    program.addOption(new Option('--method <method>', 'target method for the execute call'));

    program.action((options) => {
        const config = loadConfig(options.env);
        processCommand(options, config, getChainConfig(config, options.chainName));
    });

    program.parse();
}
