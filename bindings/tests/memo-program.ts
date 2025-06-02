import { PublicKey } from "@solana/web3.js";
import {
  axelarSolanaMemoProgramProgram,
  AXELAR_SOLANA_MEMO_PROGRAM_PROGRAM_ID,
} from "../generated/axelar-solana-memo-program/src";
import { getKeypairFromFile } from "@solana-developers/node-helpers";

describe("Ping Memo Program", () => {
  const program = axelarSolanaMemoProgramProgram();

  it("Is initialized!", async () => {
    const payer = await getKeypairFromFile();
    const [gatewayRootPdaPublicKey, _] = PublicKey.findProgramAddressSync(
      [],
      payer.publicKey
    );
    let [counterPdaPublicKey, bump] = PublicKey.findProgramAddressSync(
      [],
      AXELAR_SOLANA_MEMO_PROGRAM_PROGRAM_ID
    );

    try {
      const tx = await program.methods
        .initialize(bump)
        .accounts({
          payer: payer.publicKey,
          counterPda: counterPdaPublicKey,
        })
        .rpc();
      console.log("Your transaction signature", tx);
    } catch (error: any) {
      if (error.logs.includes("Program log: Instruction: Initialize")) {
        console.log(
          "Initializing failed, probably it has been already initialized. Skipping..."
        );
      } else {
        throw new Error("Test FAILED: Check if program is loaded");
      }
    }
    program.methods
      .processMemo("Test1")
      .accounts({ counterPda: counterPdaPublicKey })
      .rpc();
    program.methods
      .processMemo("Test2")
      .accounts({ counterPda: counterPdaPublicKey })
      .rpc();
  });
});
