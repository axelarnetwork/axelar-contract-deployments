const { Command } = require("commander");
const { mainProcessor, printWalletInfo } = require("./utils");
const { addBaseOptions } = require("./cli-utils");

async function balances(_config, wallet, client, chain) {
  await printWalletInfo(client, wallet, chain);
}

if (require.main === module) {
  const program = new Command();

  program
    .name("balances")
    .description("Display balances of the wallet on XRPL.");

  addBaseOptions(program);

  program.action((options) => {
    mainProcessor(balances, options);
  });

  program.parse();
}
