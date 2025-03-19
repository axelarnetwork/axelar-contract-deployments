const { Command, Option } = require('commander');
const { mainProcessor, hex, parseTokenAmount } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');

async function transfer(_config, wallet, client, chain, options, args) {
    await client.sendPayment(wallet, {
        destination: chain.contracts.AxelarGateway.address,
        amount: parseTokenAmount(args.token, args.amount), // token is either "XRP" or "<currency>.<issuer-address>"
        memos: [
            { memoType: hex('destination_address'), memoData: hex(args.destinationAddress.replace('0x', '')) },
            { memoType: hex('destination_chain'), memoData: hex(args.destinationChain) },
            { memoType: hex('gas_fee_amount'), memoData: Number(options.gasFeeAmount).toString(16) },
            ...(options.payload ? [{ memoType: hex('payload'), memoData: options.payload }] : []),
        ],
    }, options);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('transfer')
        .description('Initiate a token transfer and/or GMP from XRPL.')
        .arguments('<token> <amount> <destinationChain> <destinationAddress>')
        .addOption(new Option('--payload <payload>', 'payload to call contract at destination address with'))
        .addOption(new Option('--gasFeeAmount <gasFeeAmount>', 'gas fee amount').default('0'))
        .action((token, amount, destinationChain, destinationAddress, options) => {
            mainProcessor(transfer, options, { token, amount, destinationChain, destinationAddress });
        });

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.parse();
}
