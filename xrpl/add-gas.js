const { Command, Option } = require("commander");
const { mainProcessor, hex, parseTokenAmount } = require("./utils");
const { addBaseOptions, addSkipPromptOption } = require("./cli-utils");

async function addGas(_config, wallet, client, chain, options, _args) {
  await client.sendPayment(
    wallet,
    {
      destination: chain.contracts.AxelarGateway.address,
      amount: parseTokenAmount(options.token, options.amount),
      memos: [
        { memoType: hex("type"), memoData: hex("add_gas") },
        {
          memoType: hex("msg_id"),
          memoData: hex(options.msgId.toLowerCase().replace("0x", "")),
        },
      ],
    },
    options,
  );
}

if (require.main === module) {
  const program = new Command();

  program
    .name("add-gas")
    .description("Top up more gas to an XRPL message.")
    .addOption(
      new Option("--token <token>", "token to use").makeOptionMandatory(true),
    )
    .addOption(
      new Option(
        "--amount <amount>",
        "amount of gas to add",
      ).makeOptionMandatory(true),
    )
    .addOption(
      new Option(
        "--msgId <msgId>",
        "message ID whose gas to top up",
      ).makeOptionMandatory(true),
    )
    .action((options) => {
      mainProcessor(addGas, options);
    });

  addBaseOptions(program);
  addSkipPromptOption(program);

  program.parse();
}
