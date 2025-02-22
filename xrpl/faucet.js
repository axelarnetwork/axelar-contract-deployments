const xrpl = require('xrpl');
const { Command, Option } = require('commander');
const { getWallet, getAccountInfo, getFee, sendPayment, roundUpToNearestXRP } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { loadConfig, printInfo, printWarn, getChainConfig } = require('../common');

const MAX_CLAIMABLE_DROPS = 1000000000;

async function faucet(_, client, options) {
    const wallet = getWallet(options);
    const recipient = options.recipient || wallet.address;
    const { balance: recipientBalance } = await getAccountInfo(client, recipient);
    const amountInDrops = xrpl.xrpToDrops(options.amount);
    const recipientBalanceInXrp = xrpl.dropsToXrp(recipientBalance);
    const isDifferentRecipient = wallet.address.toLowerCase() !== recipient.toLowerCase();

    let fee = '0';

    if (isDifferentRecipient) {
        printInfo(`Requesting funds for`, recipient);
        fee = await getFee(client);
    }

    if (Number(recipientBalanceInXrp) >= Number(options.minBalance)) {
        printWarn(`Recipient balance (${recipientBalanceInXrp} XRP) above minimum, skipping faucet request`);
        process.exit(0);
    }

    const amountToClaim = roundUpToNearestXRP(Number(amountInDrops) + Number(fee));

    if (amountToClaim > MAX_CLAIMABLE_DROPS) {
        printWarn(`Amount too high, maximum is ${(MAX_CLAIMABLE_DROPS - fee) / 1e6} XRP`);
        process.exit(0);
    }

    await client.fundWallet(wallet, { amount: String(amountToClaim / 1e6) });

    if (isDifferentRecipient) {
        printInfo('Transferring claimed funds');
        await sendPayment(client, wallet, {
            destination: recipient,
            amount: amountInDrops,
            fee,
        });
    }

    printInfo('Funds sent', recipient);
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const client = new xrpl.Client(chain.wssRpc);
    await client.connect();

    try {
        await processor(chain, client, options);
    } finally {
        await client.disconnect();
    }
}

if (require.main === module) {
    const program = new Command();

    program
        .name('faucet')
        .addOption(new Option('--recipient <recipient>', 'recipient to request funds for'))
        .addOption(
            new Option(
                '--amount <amount>',
                'amount of XRP tokens to request from the faucet',
            ).default('100'),
        )
        .addOption(
            new Option(
                '--minBalance <amount>',
                'tokens will only be requested from the faucet if recipient XRP balance is below the amount provided',
            ).default('1'),
        )
        .description('Query the faucet for funds.')
        .action((options) => {
            mainProcessor(options, faucet);
        });

    addBaseOptions(program);

    program.parse();
}
