const { Command } = require("commander");
const { decodeTxBlob } = require("./utils");
const { printInfo, printError } = require("../common");

function processCommand(address) {
  try {
    const tx = decodeTxBlob(address);
    printInfo("Decoded transaction", tx);
  } catch (error) {
    printError("Failed to decode account ID", error.message);
    process.exit(1);
  }
}

if (require.main === module) {
  const program = new Command();

  program
    .name("decode-tx-blob")
    .description(
      "Decode XRPL serialized transaction blob into transaction object.",
    )
    .argument("<tx-blob>", "XRPL serialized transaction blob to decode");

  program.action((address) => {
    processCommand(address);
  });

  program.parse();
}
