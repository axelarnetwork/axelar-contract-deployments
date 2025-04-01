import { getKeypairFromFile } from "@solana-developers/node-helpers";
import { axelarSolanaItsProgram} from "../axelar-solana-its/src";
import { BN } from "@coral-xyz/anchor";

describe("Ping ITS", () => {
  const program = axelarSolanaItsProgram();
  const processError = (error: any, functionName: string) => {
    const errorMessage = "Program log: Instruction: " + functionName;
    if (error.logs.includes(errorMessage)) {
        console.log("Test OK: Program throws error, but data is properly sent through bindings.");
    } else {
        console.log("Test FAIL: Program throws error and data is not properly sent. Check bindings.")
    }
  };

  it("Initialize", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.initialize(
        "1", "2"
      ).accounts({
        payer: payer.publicKey,
        programDataAddress: payer.publicKey,
        itsRootPda: payer.publicKey,
        operator: payer.publicKey,
        userRolesPda: payer.publicKey,
        gatewayRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "Initialize");
    }
  })

  it("SetPauseStatus", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.setPauseStatus(
        true
      ).accounts({
        payer: payer.publicKey,
        programDataAddress: payer.publicKey,
        gatewayRootPda: payer.publicKey,
        itsRootPda: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "SetPauseStatus");
    }
  })

  it("SetTrustedChain", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.setTrustedChain(
        "chain"
      ).accounts({
        payer: payer.publicKey,
        programDataAddress: payer.publicKey,
        gatewayRootPda: payer.publicKey,
        itsRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "SetTrustedChain");
    }
  })

  it("RemoveTrustedChain", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.removeTrustedChain(
        "chain"
      ).accounts({
        payer: payer.publicKey,
        programDataAddress: payer.publicKey,
        gatewayRootPda: payer.publicKey,
        itsRootPda: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "RemoveTrustedChain");
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
      processError(error, "ApproveDeployRemoteInterchainToken");
    }
  })

  it("RevokeDeployRemoteInterchainToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.revokeDeployRemoteInterchainToken(
        payer.publicKey, [1, 2], "chain"
      ).accounts({
        payer: payer.publicKey,
        deployApprovalPda: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "RevokeDeployRemoteInterchainToken");
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
      processError(error, "RegisterCanonicalInterchainToken");
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
      processError(error, "OutboundDeploy")
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
      processError(error, "OutboundTransfer");
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
      processError(error, "InboundDeploy");
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
      processError(error, "OutboundDeploy");
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
      processError(error, "OutboundDeployMinter");
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
      processError(error, "RegisterTokenMetadata");
    }
  })

  it("RegisterCustomToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.registerCustomToken(
        [1, 2], { mintBurn: {} }, payer.publicKey
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
      processError(error, "RegisterToken");
    }
  })

  it("LinkToken", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.linkToken(
          [1, 2], "chain", Buffer.from(new Uint8Array(2)), { nativeInterchainToken: {} }, Buffer.from(new Uint8Array(3)), new BN(1), 2
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
      processError(error, "ProcessOutbound");
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
      processError(error, "OutboundTransfer");
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
      processError(error, "OutboundTransfer");
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
      processError(error, "SetFlowLimit");
    }
  })

  it("TransferOperatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.operatorTransferOperatorship({
        roles: { minter: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        gatewayRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "Operator");
    }
  })

  it("ProposeOperatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.operatorProposeOperatorship({
        roles: { operator: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        gatewayRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "Operator");
    }
  })

  it("AcceptOperatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.operatorAcceptOperatorship({
        roles: { flowLimiter: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        gatewayRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "Operator");
    }
  })

  it("TM Add Flow Limiter", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.tokenManagerAddFlowLimiter({
        roles: { flowLimiter: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "TM AddFlowLimiter");
    }
  })

  it("TM Remove Flow Limiter", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.tokenManagerRemoveFlowLimiter({
        roles: { flowLimiter: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "TM RemoveFlowLimiter");
    }
  })

  it("TM Set Flow Limit", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.tokenManagerSetFlowLimit(
        new BN(1)
      ).accounts({
        payer: payer.publicKey,
        itsRootPda: payer.publicKey,
        tokenManagerPda: payer.publicKey,
        tokenManagerUserRolesPda: payer.publicKey,
        itsUserRolesPda: payer.publicKey,
        systemProgram: payer.publicKey
      }).rpc();
    } catch (error) {
      processError(error, "TM SetFlowLimit");
    }
  })

  it("TM Transfer Operatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.tokenManagerTransferOperatorship({
        roles: { operator: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        gatewayRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "TM Operator");
    }
  })

  it("TM Propose Operatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.tokenManagerProposeOperatorship({
        roles: { operator: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        gatewayRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "TM Operator");
    }
  })

  it("TM Accept Operatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.tokenManagerAcceptOperatorship({
        roles: { operator: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        gatewayRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "TM Operator");
    }
  })

  it("TM Flow Limiter", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.tokenManagerHandOverMintAuthority(
        [1, 2]
      ).accounts({
        payer: payer.publicKey,
        mint: payer.publicKey,
        gatewayRootPda: payer.publicKey,
        itsRootPda: payer.publicKey,
        tokenManagerPda: payer.publicKey,
        minterRolesPda: payer.publicKey,
        tokenProgram: payer.publicKey,
        systemProgram: payer.publicKey
      }).rpc();
    } catch (error) {
      processError(error, "TM Hand Over Mint Authority");
    }
  })

  it("IT Mint", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.interchainTokenMint(
        new BN(1)
      ).accounts({
        mint: payer.publicKey,
        destinationAccount: payer.publicKey,
        itsRootPda: payer.publicKey,
        tokenManagerPda: payer.publicKey,
        minter: payer.publicKey,
        minterRolesPda: payer.publicKey,
        systemProgram: payer.publicKey
      }).rpc();
    } catch (error) {
      processError(error, "IT Mint");
    }
  })

  it("IT Transfer Mintership", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.interchainTokenTransferMintership({
        roles: { minter: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        gatewayRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "TM Operator");
    }
  })

  it("IT Propose Mintership", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.interchainTokenProposeMintership({
        roles: { minter: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        gatewayRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "TM Operator");
    }
  })

  it("IT Accept Mintership", async () => {
    const payer = await getKeypairFromFile();
    try {
      const tx = await program.methods.interchainTokenAcceptMintership({
        roles: { minter: {}},
        destinationRolesPdaBump: 2,
        proposalPdaBump: null
      }).accounts({
        gatewayRootPda: payer.publicKey,
        systemProgram: payer.publicKey,
        payer: payer.publicKey,
        payerRolesAccount: payer.publicKey,
        resource: payer.publicKey,
        destinationUserAccount: payer.publicKey,
        destinationRolesAccount: payer.publicKey,
        originUserAccount: payer.publicKey,
        originRolesAccount: payer.publicKey,
        proposalAccount: payer.publicKey,
      }).rpc();
    } catch (error) {
      processError(error, "TM Operator");
    }
  })
});