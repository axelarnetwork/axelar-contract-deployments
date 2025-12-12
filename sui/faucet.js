'use strict';

const { requestSuiFromFaucetV2, getFaucetHost } = require('@mysten/sui/faucet');
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
        return;
    }

    const host = options.faucet || getFaucetHost(chain.networkType);
    await requestSuiFromFaucetV2({ host, recipient });

    printInfo('Funds requested', recipient);
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
        .addOption(new Option('--faucet <faucet>', 'custom faucet rpc for Sui'))
        .description('Query the faucet for funds.')
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program);

    program.parse();
}
