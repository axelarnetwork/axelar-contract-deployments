const xrpl = require('xrpl');
const { Command, Option } = require('commander');
const { getWallet, getAccountInfo, sendTransaction, hex } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { loadConfig, printInfo, printWarn, getChainConfig } = require('../common');

async function processCommand(wallet, _, chain, args, options) {
    const client = new xrpl.Client(chain.rpc);
    await client.connect();

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
    };

    const depositTx = {
        TransactionType: 'Payment',
        Account: wallet.address,
        Amount: amount,
        Destination: chain.multisigAddress,
        Memos: [
            { Memo: { MemoType: hex('destination_address'), MemoData: args.destinationAddress.replace('0x', '') } },
            { Memo: { MemoType: hex('destination_chain'), MemoData: hex(args.destinationChain) } },
            { Memo: { MemoType: hex('gas_fee_amount'), MemoData: Number(options.gasFeeAmount).toString(16) } },
            ...(options.payload ? [{ Memo: { MemoType: hex('payload'), MemoData: options.payload } }] : []),
        ],
    };

    printInfo('Sending transaction', JSON.stringify(depositTx, null, 2));

    const txRes = await sendTransaction(client, wallet, depositTx);
    if (txRes.result.meta.TransactionResult !== 'tesSUCCESS') {
        printError('Transaction failed', txRes.result);
    } else {
        printInfo('Transaction sent');
    }

    await client.disconnect();
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);
    await processor(wallet, config, chain, args, options);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('transfer')
        .description('perform a token transfer and/or GMP')
        .arguments('<token> <amount> <destinationChain> <destinationAddress>')
        .addOption(new Option('--data <data>', 'data'))
        .addOption(new Option('--gas-fee-amount <gasFeeAmount>', 'gas fee amount').default('0'))
        .action((token, amount, destinationChain, destinationAddress, options) => {
            mainProcessor(processCommand, { token, amount, destinationChain, destinationAddress }, options);
        });

    addBaseOptions(program);

    program.parse();
}
