const { Command, Option } = require('commander');
const { ASSET_TYPE_NATIVE, getWallet, addBaseOptions, getBalances, getRpcOptions } = require('./utils');
const { loadConfig, printInfo, printWarn, getChainConfig } = require('../common');
const { Horizon, Keypair } = require('@stellar/stellar-sdk');

require('./cli-utils');

async function processCommand(chain, options) {
    const horizonServer = new Horizon.Server(chain.horizonRpc, getRpcOptions(chain));

    // For local network, ensure account always exists by funding via friendbot
    if (chain.networkType === 'local') {
        const address = Keypair.fromSecret(options.privateKey).publicKey();
        try {
            await horizonServer.friendbot(address).call();
            printInfo('Funds requested', address);
        } catch (error) {
            // Ignore errors - account already exists
        }
        return;
    }

    const keyPair = await getWallet(chain, options);
    const recipient = options.recipient || keyPair.publicKey();
    const balance = await getBalances(horizonServer, recipient).then((balances) =>
        balances.find((balance) => balance.asset_type === ASSET_TYPE_NATIVE),
    );

    if (options.recipient) {
        printInfo(`Requesting funds for`, recipient);
    }

    if (Number(balance?.balance || '0') >= Number(options.minBalance)) {
        printWarn('Wallet balance above minimum, skipping faucet request');
        process.exit(0);
    }

    await horizonServer.friendbot(recipient).call();
    printInfo('Funds requested', recipient);
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(chain, options);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('faucet')
        .addOption(new Option('--recipient <recipient>', 'recipient to request funds for'))
        .addOption(
            new Option(
                '--min-balance <amount>',
                'tokens will only be requested from the faucet if recipient balance is below the amount provided',
            ).default('1'),
        )
        .description('Query the faucet for funds.')
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program);

    program.parse();
}
