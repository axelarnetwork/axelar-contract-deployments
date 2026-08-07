const { Command, Option } = require('commander');
const { mainProcessor, hex, parseTokenAmount, encodeITSDestination } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');

async function interchainTransfer(config, wallet, client, chain, options, args) {
    // Two encoding layers: encodeITSDestination produces the canonical raw-bytes hex
    // for the destination chain type, then hex() wraps it for XRPL memo transport.
    const destinationAddress = encodeITSDestination(config.chains, args.destinationChain, args.destinationAddress);

    // The gas fee is always denominated in the same token as the transfer, so parse it the
    // same way as the amount: XRP is converted to a drops string, IOUs to a decimal string.
    // The gas_fee_amount memo carries the wire value (u64 drops for XRP, decimal for IOUs).
    const parsedGasFee = parseTokenAmount(args.token, options.gasFeeAmount);
    const gasFeeAmount = typeof parsedGasFee === 'string' ? parsedGasFee : parsedGasFee.value;

    await client.sendPayment(
        wallet,
        {
            destination: chain.contracts.InterchainTokenService.address,
            amount: parseTokenAmount(args.token, args.amount), // token is either "XRP" or "<currency>.<issuer-address>"
            memos: [
                { memoType: hex('type'), memoData: hex('interchain_transfer') },
                { memoType: hex('destination_address'), memoData: hex(destinationAddress.replace('0x', '')) },
                { memoType: hex('destination_chain'), memoData: hex(args.destinationChain) },
                { memoType: hex('gas_fee_amount'), memoData: hex(gasFeeAmount) },
                ...(options.payload ? [{ memoType: hex('payload'), memoData: options.payload }] : []),
            ],
        },
        options,
    );
}

if (require.main === module) {
    const program = new Command();

    program
        .name('interchain-transfer')
        .description('Initiate an interchain token transfer from XRPL.')
        .arguments('<token> <amount> <destinationChain> <destinationAddress>')
        .addOption(new Option('--payload <payload>', 'payload to call contract at destination address with'))
        .addOption(
            new Option(
                '--gasFeeAmount <gasFeeAmount>',
                'gas fee amount, denominated in the transferred token (e.g. XRP, not drops)',
            ).makeOptionMandatory(true),
        )
        .action((token, amount, destinationChain, destinationAddress, options) => {
            return mainProcessor(interchainTransfer, options, { token, amount, destinationChain, destinationAddress });
        });

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.parseAsync().then(() => process.exit(0));
}
