const xrpl = require('xrpl');
const { Command, Option } = require('commander');
const { getWallet, sendPayment, hex } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { loadConfig, getChainConfig } = require('../common');

function parseAmount(args) {
    let amount;

    if (args.token === 'XRP') {
        amount = xrpl.xrpToDrops(args.amount);
    } else {
        const [currency, issuer] = args.token.split('.');
        amount = {
            currency,
            issuer,
            value: args.amount,
        };
    }

    return amount;
}

async function transfer(wallet, _, client, chain, args, options) {
    await sendPayment(client, wallet, {
        destination: chain.multisigAddress,
        amount: parseAmount(args), // args.token is either XRP or IOU.<issuer-address>
        memos: [
            { memoType: hex('destination_address'), memoData: args.destinationAddress.replace('0x', '') },
            { memoType: hex('destination_chain'), memoData: hex(args.destinationChain) },
            { memoType: hex('gas_fee_amount'), memoData: Number(options.gasFeeAmount).toString(16) },
            ...(options.payload ? [{ memoType: hex('payload'), memoData: options.payload }] : []),
        ],
    });
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = getWallet(options);
    const client = new xrpl.Client(chain.wssRpc);
    await client.connect();

    try {
        await processor(wallet, config, client, chain, args, options);
    } finally {
        await client.disconnect();
    }
}

if (require.main === module) {
    const program = new Command();

    program
        .name('transfer')
        .description('initiate a token transfer and/or GMP from XRPL')
        .arguments('<token> <amount> <destinationChain> <destinationAddress>')
        .addOption(new Option('--payload <payload>', 'payload to call contract at destination address with'))
        .addOption(new Option('--gas-fee-amount <gasFeeAmount>', 'gas fee amount').default('0'))
        .action((token, amount, destinationChain, destinationAddress, options) => {
            mainProcessor(transfer, { token, amount, destinationChain, destinationAddress }, options);
        });

    addBaseOptions(program);

    program.parse();
}
