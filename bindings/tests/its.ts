import { getKeypairFromFile } from "@solana-developers/node-helpers";
import { axelarSolanaItsProgram } from "../generated/axelar-solana-its/src";
import { BN } from "@coral-xyz/anchor";
import {
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";
import { TOKEN_METADATA_PROGRAM_ID } from "../axelar-solana-its/src/pda";

describe("Ping ITS", () => {
  const program = axelarSolanaItsProgram();
  const systemAccount = SystemProgram.programId.toBase58();
  const processError = (error: any, functionName: string) => {
    const errorMessage = "Program log: Instruction: " + functionName;
    if (error.logs.includes(errorMessage)) {
      console.log(
        "Test OK: Program throws error, but data is properly sent through bindings."
      );
    } else {
      throw new Error(
        "Test FAIL: Program throws error and data is not properly sent. Check bindings."
      );
    }
  };

  it("Initialize", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .initialize("1", "2")
        .accounts({
          payer: payer.publicKey,
          programDataAddress: payer.publicKey,
          itsRootPda: payer.publicKey,
          operator: payer.publicKey,
          userRolesPda: payer.publicKey,
          systemProgram: systemAccount,
        })
        .rpc();
    } catch (error) {
      processError(error, "Initialize");
    }
  });

  it("SetPauseStatus", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .setPauseStatus(true)
        .accounts({
          payer: payer.publicKey,
          programDataAddress: payer.publicKey,
          itsRootPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "SetPauseStatus");
    }
  });

  it("SetTrustedChain", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .setTrustedChain("chain")
        .accounts({
          payer: payer.publicKey,
          programDataAddress: payer.publicKey,
          itsRootPda: payer.publicKey,
          systemProgram: systemAccount,
        })
        .rpc();
    } catch (error) {
      processError(error, "SetTrustedChain");
    }
  });

  it("RemoveTrustedChain", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .removeTrustedChain("chain")
        .accounts({
          payer: payer.publicKey,
          programDataAddress: payer.publicKey,
          itsRootPda: payer.publicKey,
          systemProgram: systemAccount,
        })
        .rpc();
    } catch (error) {
      processError(error, "RemoveTrustedChain");
    }
  });

  it("ApproveDeployRemoteInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .approveDeployRemoteInterchainToken(
          payer.publicKey,
          [1, 2],
          "chain",
          Buffer.from(new Uint8Array(2))
        )
        .accounts({
          payer: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          rolesPda: payer.publicKey,
          deployApprovalPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "ApproveDeployRemoteInterchainToken");
    }
  });

  it("RevokeDeployRemoteInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .revokeDeployRemoteInterchainToken(payer.publicKey, [1, 2], "chain")
        .accounts({
          payer: payer.publicKey,
          deployApprovalPda: payer.publicKey,
          systemProgram: systemAccount,
        })
        .rpc();
    } catch (error) {
      processError(error, "RevokeDeployRemoteInterchainToken");
    }
  });

  it("RegisterCanonicalInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .registerCanonicalInterchainToken()
        .accounts({
          payer: payer.publicKey,
          tokenMetadataAccount: payer.publicKey,
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          mint: payer.publicKey,
          tokenManagerAta: payer.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          splAssociatedTokenAccount: ASSOCIATED_TOKEN_PROGRAM_ID,
          itsUserRolesPda: payer.publicKey,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .rpc();
    } catch (error) {
      processError(error, "RegisterToken");
    }
  });

  it("DeployRemoteCanonicalInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .deployRemoteCanonicalInterchainToken("chain", new BN(1), 2)
        .accounts({
          payer: payer.publicKey,
          mint: payer.publicKey,
          metadataAccount: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          axelarSolanaGateway: payer.publicKey,
          gasConfigPda: payer.publicKey,
          gasService: payer.publicKey,
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          callContractSigningPda: payer.publicKey,
          id: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "OutboundDeploy");
    }
  });

  it("InterchainTransfer", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .interchainTransfer(
          [1, 2],
          "chain",
          Buffer.from(new Uint8Array(1)),
          new BN(2),
          new BN(3),
          4
        )
        .accounts({
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
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          callContractSigningPda: payer.publicKey,
          id: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "OutboundTransfer");
    }
  });

  it("DeployInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .deployInterchainToken([1, 2], "name", "symbol", 2, new BN(0))
        .accounts({
          payer: payer.publicKey,
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          mint: payer.publicKey,
          tokenManagerAta: payer.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          splAssociatedTokenAccount: ASSOCIATED_TOKEN_PROGRAM_ID,
          itsUserRolesPda: payer.publicKey,
          rent: SYSVAR_RENT_PUBKEY,
          sysvarInstructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          mplTokenMetadata: TOKEN_METADATA_PROGRAM_ID,
          metadataAccount: payer.publicKey,
          payerAta: payer.publicKey,
          optionalMinterRolesPda: payer.publicKey,
          optionalMinter: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "InboundDeploy");
    }
  });

  it("DeployRemoteInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .deployRemoteInterchainToken([1, 2], "chain", new BN(1), 2)
        .accounts({
          payer: payer.publicKey,
          mint: payer.publicKey,
          metadataAccount: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          axelarSolanaGateway: payer.publicKey,
          gasConfigPda: payer.publicKey,
          gasService: payer.publicKey,
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          callContractSigningPda: payer.publicKey,
          id: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "OutboundDeploy");
    }
  });

  it("DeployRemoteInterchainTokenWithMinter", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .deployRemoteInterchainTokenWithMinter(
          [1, 2],
          "chain",
          Buffer.from(new Uint8Array(1)),
          new BN(4),
          5
        )
        .accounts({
          payer: payer.publicKey,
          mint: payer.publicKey,
          metadataAccount: payer.publicKey,
          minter: payer.publicKey,
          deployApproval: payer.publicKey,
          minterRolesPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          axelarSolanaGateway: payer.publicKey,
          gasConfigPda: payer.publicKey,
          gasService: payer.publicKey,
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          callContractSigningPda: payer.publicKey,
          id: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "OutboundDeployMinter");
    }
  });

  it("RegisterTokenMetadata", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .registerTokenMetadata(new BN(1), 2)
        .accounts({
          payer: payer.publicKey,
          mint: payer.publicKey,
          tokenProgram: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          axelarSolanaGateway: payer.publicKey,
          gasConfigPda: payer.publicKey,
          gasService: payer.publicKey,
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          callContractSigningPda: payer.publicKey,
          id: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "RegisterTokenMetadata");
    }
  });

  it("RegisterCustomToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .registerCustomToken([1, 2], { mintBurn: {} }, payer.publicKey)
        .accounts({
          payer: payer.publicKey,
          tokenMetadataAccount: payer.publicKey,
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          mint: payer.publicKey,
          tokenManagerAta: payer.publicKey,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          splAssociatedTokenAccount: ASSOCIATED_TOKEN_PROGRAM_ID,
          itsUserRolesPda: payer.publicKey,
          rent: SYSVAR_RENT_PUBKEY,
          optionalOperator: payer.publicKey,
          optionalOperatorRolesPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "RegisterToken");
    }
  });

  it("LinkToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .linkToken(
          [1, 2],
          "chain",
          Buffer.from(new Uint8Array(2)),
          { nativeInterchainToken: {} },
          Buffer.from(new Uint8Array(3)),
          new BN(1),
          2
        )
        .accounts({
          payer: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          gatewayRootPda: payer.publicKey,
          axelarSolanaGateway: payer.publicKey,
          gasConfigPda: payer.publicKey,
          gasService: payer.publicKey,
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          callContractSigningPda: payer.publicKey,
          id: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "ProcessOutbound");
    }
  });

  it("CallContractWithInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .callContractWithInterchainToken(
          [1, 2],
          "chain",
          Buffer.from(new Uint8Array(1)),
          new BN(1),
          Buffer.from(new Uint8Array(3)),
          new BN(4),
          2
        )
        .accounts({
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
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          callContractSigningPda: payer.publicKey,
          id: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "OutboundTransfer");
    }
  });

  it("CallContractWithInterchainTokenOffchainData", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .callContractWithInterchainTokenOffchainData(
          [1, 2],
          "chain",
          Buffer.from(new Uint8Array(1)),
          new BN(1),
          [3, 4],
          new BN(4),
          2
        )
        .accounts({
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
          systemProgram: systemAccount,
          itsRootPda: payer.publicKey,
          callContractSigningPda: payer.publicKey,
          id: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "OutboundTransfer");
    }
  });

  it("SetFlowLimit", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .setFlowLimit(new BN(1))
        .accounts({
          payer: payer.publicKey,
          itsRootPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          itsUserRolesPda: payer.publicKey,
          tokenManagerUserRolesPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "SetFlowLimit");
    }
  });

  it("TransferOperatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .transferOperatorship()
        .accounts({
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          itsRootPda: payer.publicKey,
          to: payer.publicKey,
          destinationRolesPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "TransferOperatorship");
    }
  });

  it("ProposeOperatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .proposeOperatorship()
        .accounts({
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          itsRootPda: payer.publicKey,
          to: payer.publicKey,
          destinationRolesPda: payer.publicKey,
          proposalPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "ProposeOperatorship");
    }
  });

  it("AcceptOperatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .acceptOperatorship()
        .accounts({
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          itsRootPda: payer.publicKey,
          from: payer.publicKey,
          originRolesPda: payer.publicKey,
          proposalPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "AcceptOperatorship");
    }
  });

  it("TM Add Flow Limiter", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .addTokenManagerFlowLimiter()
        .accounts({
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          flowLimiter: payer.publicKey,
          flowLimiterRolesPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "AddTokenManagerFlowLimiter");
    }
  });

  it("TM Remove Flow Limiter", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .removeTokenManagerFlowLimiter()
        .accounts({
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          flowLimiter: payer.publicKey,
          flowLimiterRolesPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "RemoveTokenManagerFlowLimiter");
    }
  });

  it("TM Set Flow Limit", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .setTokenManagerFlowLimit(new BN(1))
        .accounts({
          payer: payer.publicKey,
          itsRootPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          tokenManagerUserRolesPda: payer.publicKey,
          itsUserRolesPda: payer.publicKey,
          systemProgram: systemAccount,
        })
        .rpc();
    } catch (error) {
      processError(error, "SetTokenManagerFlowLimit");
    }
  });

  it("TM Transfer Operatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .transferTokenManagerOperatorship()
        .accounts({
          itsRootPda: payer.publicKey,
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          to: payer.publicKey,
          destinationRolesPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "TransferTokenManagerOperatorship");
    }
  });

  it("TM Propose Operatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .proposeTokenManagerOperatorship()
        .accounts({
          itsRootPda: payer.publicKey,
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          to: payer.publicKey,
          destinationRolesPda: payer.publicKey,
          proposalPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "ProposeTokenManagerOperatorship");
    }
  });

  it("TM Accept Operatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .acceptTokenManagerOperatorship()
        .accounts({
          itsRootPda: payer.publicKey,
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          from: payer.publicKey,
          originRolesPda: payer.publicKey,
          proposalPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "AcceptTokenManagerOperatorship");
    }
  });

  it("Handover Mint Authority", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .handoverMintAuthority([1, 2])
        .accounts({
          payer: payer.publicKey,
          mint: payer.publicKey,
          itsRootPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          minterRolesPda: payer.publicKey,
          tokenProgram: payer.publicKey,
          systemProgram: systemAccount,
        })
        .rpc();
    } catch (error) {
      processError(error, "HandoverMintAuthority");
    }
  });

  it("IT Mint", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .mintInterchainToken(new BN(1))
        .accounts({
          mint: payer.publicKey,
          to: payer.publicKey,
          itsRootPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          minter: payer.publicKey,
          minterRolesPda: payer.publicKey,
          tokenProgram: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "MintInterchainToken");
    }
  });

  it("IT Transfer Mintership", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .transferInterchainTokenMintership()
        .accounts({
          itsRootPda: payer.publicKey,
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          to: payer.publicKey,
          destinationRolesPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "TransferInterchainTokenMintership");
    }
  });

  it("IT Propose Mintership", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .proposeInterchainTokenMintership()
        .accounts({
          itsRootPda: payer.publicKey,
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          to: payer.publicKey,
          destinationRolesPda: payer.publicKey,
          proposalPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "ProposeInterchainTokenMintership");
    }
  });

  it("IT Accept Mintership", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods
        .acceptInterchainTokenMintership()
        .accounts({
          itsRootPda: payer.publicKey,
          systemProgram: systemAccount,
          payer: payer.publicKey,
          payerRolesPda: payer.publicKey,
          tokenManagerPda: payer.publicKey,
          from: payer.publicKey,
          originRolesPda: payer.publicKey,
          proposalPda: payer.publicKey,
        })
        .rpc();
    } catch (error) {
      processError(error, "AcceptInterchainTokenMintership");
    }
  });
});
