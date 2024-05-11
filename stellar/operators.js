const { Contract, Address } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { getWallet, prepareTransaction, buildTransaction, sendTransaction, estimateCost } = require('./utils');
const { loadConfig, printInfo } = require('../evm/utils');
require('./cli-utils');


async function processCommand(options, _, chain) {
    const [wallet, server] = await getWallet(chain, options);

    const contract = new Contract(options.address || chain.contracts?.axelar_operators?.address);

    let operation;

    const address = Address.fromString(options.args || wallet.publicKey());

    switch (options.action) {
        case 'is_operator': {
            operation = contract.call('is_operator', address.toScVal());
            break;
        }
        case 'add_operator': {
            operation = contract.call('add_operator', address.toScVal());
            break;
        }
        case 'remove_operator': {
            operation = contract.call('remove_operator', address.toScVal());
            break;
        }
        default: {
            throw new Error(`Unknown action: ${options.action}`);
        }
    }

    if (options.estimateCost) {
        const tx = await buildTransaction(operation, server, wallet, chain.networkType, options);
        await estimateCost(tx, server);
        return;
    }

    const signedTx = await prepareTransaction(operation, server, wallet, chain.networkType, options);
    const returnValue = await sendTransaction(signedTx, server);
    printInfo('is_operator', returnValue);
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
            .choices(['is_operator', 'add_operator', 'remove_operator'])
            .makeOptionMandatory(true),
    );
    program.addOption(new Option('--estimateCost', 'estimate on-chain resources').default(false));
    program.addOption(new Option('--args <args>', 'arguments for the contract call'));
    program.addOption(new Option('--address <address>', 'contract address'));

    program.action((options) => {
        const config = loadConfig(options.env);
        processCommand(options, config, config.stellar);
    });

    program.parse();
}
