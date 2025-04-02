const { Command, Option } = require('commander');
const { mainProcessor, hex, parseTokenAmount } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');

async function callContract(_config, wallet, client, chain, options, args) {
    await client.sendPayment(
        wallet,
        {
            destination: chain.contracts.AxelarGateway.address,
            amount: parseTokenAmount(options.gasFeeToken, options.gasFeeAmount), // token is either "XRP" or "<currency>.<issuer-address>"
            memos: [
                { memoType: hex('type'), memoData: hex('call_contract') },
                { memoType: hex('destination_address'), memoData: hex(args.destinationAddress.replace('0x', '')) },
                { memoType: hex('destination_chain'), memoData: hex(args.destinationChain) },
                { memoType: hex('payload'), memoData: options.payload },
            ],
        },
        options,
    );
}

if (require.main === module) {
    const program = new Command();

    program
        .name('call-contract')
        .description('Initiate a GMP call from XRPL.')
        .arguments('<destinationChain> <destinationAddress>')
        .addOption(new Option('--payload <payload>', 'payload to call contract at destination address with').makeOptionMandatory(true))
        .addOption(
            new Option('--gasFeeToken <gasFeeToken>', 'token to pay gas in ("XRP" or "<currency>.<issuer>")').makeOptionMandatory(true),
        )
        .addOption(
            new Option('--gasFeeAmount <gasFeeAmount>', 'amount of the deposited tokens that will be used to pay gas').makeOptionMandatory(
                true,
            ),
        )
        .action((destinationChain, destinationAddress, options) => {
            mainProcessor(callContract, options, { destinationChain, destinationAddress });
        });

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.parse();
}
