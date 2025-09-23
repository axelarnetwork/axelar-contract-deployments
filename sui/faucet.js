'use strict';

const { requestSuiFromFaucetV0, requestSuiFromFaucetV2, getFaucetHost } = require('@mysten/sui/faucet');
const { saveConfig, loadConfig, printInfo, printWarn, getChainConfig } = require('../common/utils');
const { getWallet, printWalletInfo, addBaseOptions } = require('./utils');
const { Command, Option } = require('commander');

async function processCommand(config, chain, options) {
    const [keypair, client] = getWallet(chain, options);
    const recipient = options.recipient || keypair.toSuiAddress();

    await printWalletInfo(recipient, client, chain, options);

    const balance = Number((await client.getBalance({ owner: recipient })).totalBalance) / 1e9;

    if (balance >= Number(options.minBalance)) {
        printWarn('Wallet balance above minimum, skipping faucet request');
        process.exit(0);
    }

    const faucetHost = getFaucetHost(chain.networkType);

    try {
        // Use V0 for localnet, V2 for other networks
        if (chain.networkType === 'localnet') {
            await requestSuiFromFaucetV0({
                host: faucetHost,
                recipient,
            });
        } else {
            await requestSuiFromFaucetV2({
                host: faucetHost,
                recipient,
            });
        }
        printInfo('Funds requested', recipient);
    } catch (error) {
        printWarn(`Failed to request funds from faucet at ${faucetHost}: ${error.message}`);
        throw error;
    }
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(config, chain, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('faucet')
        .addOption(new Option('--recipient <recipient>', 'recipient to request funds for'))
        .addOption(
            new Option(
                '--minBalance <amount>',
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
