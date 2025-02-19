const xrpl = require('xrpl');
const { Command, Option } = require('commander');
const { getWallet, getAccountInfo, getFee, sendTransaction } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { loadConfig, printInfo, printWarn, getChainConfig } = require('../common');

const MAX_DROPS_AMOUNT = 1_000_000_000;

async function processCommand(chain, options) {
    const wallet = await getWallet(chain, options);
    const recipient = options.recipient || wallet.address;

    const isDifferentRecipient = wallet.address.toLowerCase() !== recipient.toLowerCase();
    if (isDifferentRecipient) {
        printInfo(`Requesting funds for`, recipient);
    }

    const client = new xrpl.Client(chain.rpc);
    await client.connect();

    try {
        const balance = Number((await getAccountInfo(client, recipient)).Balance) / 1e6;
        if (balance >= Number(options.minBalance)) {
            printWarn(`Recipient balance (${balance} XRP) above minimum, skipping faucet request`);
            process.exit(0);
        }
    } catch (error) {
        if (error.data.error !== 'actNotFound') {
            printWarn('Failed to get account info for recipient', recipient);
            throw error;
        }
    }

    const fee = isDifferentRecipient ? await getFee(client) : '0';

    const amountInDrops = xrpl.xrpToDrops(options.amount);
    const amountToClaim = Number(amountInDrops) + Number(fee);
    if (amountToClaim > MAX_DROPS_AMOUNT) {
        printWarn(`Amount too high, maximum is ${(MAX_DROPS_AMOUNT - fee) / 1e6} XRP`);
        process.exit(0);
    }

    await client.fundWallet(wallet, { amount: String(amountToClaim / 1e6) });
    if (isDifferentRecipient) {
        const paymentTx = {
            TransactionType: 'Payment',
            Account: wallet.address,
            Destination: recipient,
            Amount: amountInDrops,
            Fee: fee,
        };

        printInfo('Transferring claimed funds', JSON.stringify(paymentTx, null, 2));
        await sendTransaction(client, wallet, paymentTx);
    }

    await client.disconnect();

    printInfo('Funds sent', recipient);
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
                '--amount <amount>',
                'amount of tokens to request from the faucet',
            ).default('100'),
        )
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
