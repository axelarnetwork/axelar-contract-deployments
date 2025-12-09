const { Command, Option } = require('commander');
const { ASSET_TYPE_NATIVE, getWallet, addBaseOptions, getBalances, getRpcOptions } = require('./utils');
const { loadConfig, printInfo, printWarn, printError, getChainConfig } = require('../common');
const { Horizon, Keypair } = require('@stellar/stellar-sdk');

require('./cli-utils');

async function processCommand(chain, options) {
    const horizonServer = new Horizon.Server(chain.horizonRpc, getRpcOptions(chain));
    const isLocalNetwork = chain.networkType === 'local';
    const recipient =
        options.recipient ||
        (isLocalNetwork ? Keypair.fromSecret(options.privateKey).publicKey() : (await getWallet(chain, options)).publicKey());

    // For non-local networks, check balance before requesting funds
    if (!isLocalNetwork) {
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
    }

    try {
        await horizonServer.friendbot(recipient).call();
        printInfo('Funds requested', recipient);
    } catch (error) {
        // Friendbot typically returns 400 status when account is already funded
        if (error?.response?.status === 400) {
            printWarn('Account already funded', recipient);
            if (!isLocalNetwork) throw error; // Only swallow on local network
        } else {
            printError('Friendbot request failed', {
                recipient,
                status: error?.response?.status,
                message: error?.message || error,
            });
            throw error;
        }
    }
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
