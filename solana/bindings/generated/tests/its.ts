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

  it("InterchainTransfer", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.interchainTransfer(
          [1, 2], "chain", Buffer.from(new Uint8Array(1)), new BN(2), new BN(3), 4
        ).accounts({
          payer: payer.publicKey,
          authority: payer.publicKey,
          sourceAccount: payer.publicKey,
          mint: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          tokenManagerAta: payer.publicKey,
          tokenProgram: payer.publicKey,
          flowSlotPda: payer.publicKey,
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
        console.log("Error from InterchainTransfer");
    }
  })

  it("DeployInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.deployInterchainToken(
          [1, 2], "name", "symbol", 2
        ).accounts({
          payer: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          systemProgram: payer.publicKey,
          itsRootPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          mint: payer.publicKey,
          tokenManagerAta: payer.publicKey,
          tokenProgram: payer.publicKey,
          splAssociatedTokenAccount: payer.publicKey,
          itsUserRolesPda: payer.publicKey,
          rent: payer.publicKey,
          sysvarInstructions: payer.publicKey,
          mplTokenMetadata: payer.publicKey,
          metadataAccount: payer.publicKey,
          minter: payer.publicKey,
          minterRolesPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from DeployInterchainToken");
    }
  })

  it("DeployRemoteInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.deployRemoteInterchainToken(
          [1, 2], "chain", new BN(1), 2
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
          id: payer.publicKey
        }).rpc();
    } catch (error) {
        console.log("Error from DeployRemoteInterchainToken");
    }
  })

  it("DeployRemoteInterchainTokenWithMinter", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.deployRemoteInterchainTokenWithMinter(
          [1, 2], "chain", Buffer.from(new Uint8Array(1)), new BN(4), 5
        ).accounts({
          payer: payer.publicKey,
          mint: payer.publicKey,
          metadataAccount: payer.publicKey,
          minter: payer.publicKey,
          deployApproval: payer.publicKey,
          minterRolesPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          sysvarInstructions: payer.publicKey,
          mplTokenMetadata: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          axelarSolanaGateway: payer.publicKey,
          gasConfigPda: payer.publicKey,
          gasService: payer.publicKey,
          systemProgram: payer.publicKey,
          itsRootPda: payer.publicKey,
          callContractSigningPda: payer.publicKey,
          id: payer.publicKey
        }).rpc();
    } catch (error) {
        console.log("Error from DeployRemoteInterchainTokenWithMinter");
    }
  })

  it("RegisterTokenMetadata", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.registerTokenMetadata(
          new BN(1), 2
        ).accounts({
          payer: payer.publicKey,
          mint: payer.publicKey,
          tokenProgram: payer.publicKey,
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
        console.log("Error from RegisterTokenMetadata");
    }
  })

  it("RegisterCustomToken", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.registerCustomToken(
          [1, 2], {nativeInterchainToken: 1}, payer.publicKey
        ).accounts({
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
          rent: payer.publicKey,
          operator: payer.publicKey,
          operatorRolesPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from RegisterCustomToken");
    }
  })

  it("LinkToken", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.linkToken(
           [1, 2], "chain", Buffer.from(new Uint8Array(2)), { mintBurn: 1 }, Buffer.from(new Uint8Array(3)), new BN(1), 2
        ).accounts({
          payer: payer.publicKey,
          tokenManagerPda: payer.publicKey,
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
        console.log("Error from LinkToken");
    }
  })

  it("CallContractWithInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.callContractWithInterchainToken(
          [1, 2], "chain", Buffer.from(new Uint8Array(1)), new BN(1),
          Buffer.from(new Uint8Array(3)), new BN(4), 2
        ).accounts({
          payer: payer.publicKey,
          authority: payer.publicKey,
          sourceAccount: payer.publicKey,
          mint: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          tokenManagerAta: payer.publicKey,
          tokenProgram: payer.publicKey,
          flowSlotPda: payer.publicKey,
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
        console.log("Error from CallContractWithInterchainToken");
    }
  })

  it("CallContractWithInterchainTokenOffchainData", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.callContractWithInterchainTokenOffchainData(
          [1, 2], "chain", Buffer.from(new Uint8Array(1)), new BN(1),
          [3, 4], new BN(4), 2
        ).accounts({
          payer: payer.publicKey,
          authority: payer.publicKey,
          sourceAccount: payer.publicKey,
          mint: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          tokenManagerAta: payer.publicKey,
          tokenProgram: payer.publicKey,
          flowSlotPda: payer.publicKey,
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
        console.log("Error from CallContractWithInterchainTokenOffchainData");
    }
  })

  it("SetFlowLimit", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.setFlowLimit(
          new BN(1)
        ).accounts({
          payer: payer.publicKey,
          itsRootPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          itsUserRolesPda: payer.publicKey,
          tokenManagerUserRolesPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        console.log("Error from SetFlowLimit");
    }
  })
});