const xrpl = require('xrpl');
const { Command, Option } = require('commander');
const { getWallet, getAccountInfo, sendTransaction } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { loadConfig, printInfo, printWarn, getChainConfig } = require('../common');

async function processCommand(chain, options) {
    const wallet = await getWallet(chain, options);
    const recipient = options.recipient || wallet.address;

    if (wallet.address.toLowerCase() !== options?.recipient?.toLowerCase()) {
        printInfo(`Requesting funds for`, recipient);
    }

    const client = new xrpl.Client(chain.rpc);
    await client.connect();

    try {
        const balance = Number((await getAccountInfo(client, recipient)).Balance) / 1e6;
        if (balance >= Number(options.minBalance)) {
            printWarn('Wallet balance above minimum, skipping faucet request');
            process.exit(0);
        }
    } catch (error) {
        if (error.data.error !== 'actNotFound') {
            printWarn('Failed to get account info for recipient', recipient);
            throw error;
        }
    }

    await client.fundWallet(wallet, { amount: "105" });
    if (wallet.address.toLowerCase() !== recipient.toLowerCase()) {
        const paymentTx = {
            TransactionType: 'Payment',
            Account: wallet.address,
            Destination: recipient,
            Amount: xrpl.xrpToDrops("100"), // TODO: Subtract actual fee.
        };

        await sendTransaction(client, wallet, paymentTx);
    }

    await client.disconnect();

    printInfo('Funds requested', recipient);
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(chain, options);
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
