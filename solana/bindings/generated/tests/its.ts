import { getKeypairFromFile } from "@solana-developers/node-helpers";
import { axelarSolanaItsProgram} from "../axelar-solana-its/src";
import { BN } from "@coral-xyz/anchor";

describe("Ping ITS", () => {
  const program = axelarSolanaItsProgram();

  it("Initialize", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.initialize("1", "2").accounts({
            payer: payer.publicKey,
            programDataAddress: payer.publicKey,
            itsRootPda: payer.publicKey,
            operator: payer.publicKey,
            userRolesPda: payer.publicKey,
            gatewayRootPda: payer.publicKey,
            systemProgram: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from Initialize");
    }
  })

  it("SetPauseStatus", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.setPauseStatus(true).accounts({
          payer: payer.publicKey,
          programDataAddress: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          itsRootPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from SetPauseStatus");
    }
  })

  it("SetTrustedChain", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.setTrustedChain("chain").accounts({
          payer: payer.publicKey,
          programDataAddress: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          itsRootPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from SetTrustedChain");
    }
  })

  it("RemoveTrustedChain", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.removeTrustedChain("chain").accounts({
          payer: payer.publicKey,
          programDataAddress: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          itsRootPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from RemoveTrustedChain");
    }
  })

  it("ApproveDeployRemoteInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.approveDeployRemoteInterchainToken(
          payer.publicKey, [1, 2], "chain", Buffer.from(new Uint8Array(2))
        ).accounts({
          payer: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          rolesPda: payer.publicKey,
          deployApprovalPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from ApproveDeployRemoteInterchainToken");
    }
  })

  it("RevokeDeployRemoteInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.revokeDeployRemoteInterchainToken(
          payer.publicKey, [1, 2], "chain"
        ).accounts({
          payer: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          rolesPda: payer.publicKey,
          deployApprovalPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from RevokeDeployRemoteInterchainToken");
    }
  })

  it("RegisterCanonicalInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.registerCanonicalInterchainToken().accounts({
          payer: payer.publicKey,
          tokenMetadataAccount: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          systemProgram: payer.publicKey,
          itsRootPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          mint: payer.publicKey,
          tokenManagerAta: payer.publicKey,
          tokenProgram: payer.publicKey,
          splAssociatedTokenAccount: payer.publicKey,
          itsUserRolesPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from RegisterCanonicalInterchainToken");
    }
  })

  it("DeployRemoteCanonicalInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.deployRemoteCanonicalInterchainToken(
          "chain", new BN(1), 2
        ).accounts({
          payer: payer.publicKey,
          mint: payer.publicKey,
          metadataAccount: payer.publicKey,
          sysvarInstructions: payer.publicKey,
          mplTokenMetadata: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          axelarSolanaGateway: payer.publicKey,
          gasConfigPda: payer.publicKey,
          gasService: payer.publicKey,
          systemProgram: payer.publicKey,
          itsRootPda: payer.publicKey,
          callContractSigningPda: payer.publicKey,
          id: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from DeployRemoteCanonicalInterchainToken");
    }
  })
});