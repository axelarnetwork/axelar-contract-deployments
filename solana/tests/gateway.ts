import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Gateway } from "../target/types/gateway";

describe("axelar gateway for solana", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Gateway as Program<Gateway>;

  it("call_contract", async () => {
    // Add your test here.
    const tx = await program.methods.callContract
    console.log("Your transaction signature", tx);
  });
});
