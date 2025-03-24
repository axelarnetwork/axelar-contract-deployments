import { getKeypairFromFile } from "@solana-developers/node-helpers";
import { axelarSolanaGatewayProgram} from "../axelar-solana-gateway/src";
import { Keypair, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

describe("Ping Gateway", () => {
  const program = axelarSolanaGatewayProgram();
  const processError = (error: any, functionName: string) => {
    const errorMessage = "Program log: Instruction: " + functionName;
    if (error.logs.includes(errorMessage)) {
        console.log("Test OK: Program throws error, but data is properly sent through bindings.");
    } else {
        console.log("Test FAIL: Program throws error and data is not properly sent. Check bindings.")
    }
  };
  it("ApproveMessage", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.approveMessage({
            leaf: {
                    message: {
                        ccId: {
                            chain: "",
                            id: "",
                        },
                        sourceAddress: "",
                        destinationChain: "",
                        destinationAddress: "",
                        payloadHash: [1],
                    },
                    position: 4,
                    setSize: 5,
                    domainSeparator: [4],
                    signingVerifierSet: [6],
                },
                proof: Buffer.from(new Uint8Array(1)),
            }, [1]).accounts({
            gatewayRootPda: payer.publicKey,
            payer: payer.publicKey,
            verificationSessionPda: payer.publicKey,
            incomingMessagePda: payer.publicKey,
        }).rpc();
    } catch (error) {
        processError(error, "Approve Messages");
    }
  })

  it("RotateSigners", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.rotateSigners([1]).accounts({
            gatewayRootPda: payer.publicKey,
            verificationSessionAccount: payer.publicKey,
            currentVerifierSetTrackerPda: payer.publicKey,
            newVerifierSetTrackerPda: payer.publicKey,
            payer: payer.publicKey,
            operator: null,
        }).rpc();
    } catch (error) {
        processError(error, "Rotate Signers");
    }
  })

  it("RotateSigners with Optional", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.rotateSigners([1]).accounts({
            gatewayRootPda: payer.publicKey,
            verificationSessionAccount: payer.publicKey,
            currentVerifierSetTrackerPda: payer.publicKey,
            newVerifierSetTrackerPda: payer.publicKey,
            payer: payer.publicKey,
            operator: payer.publicKey,
        }).rpc();
    } catch (error) {
        processError(error, "Rotate Signers");
    }
  })

  it("CallContract", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.callContract("1", "2", Buffer.from(new Uint8Array(2)), 1).accounts({
            senderProgram: payer.publicKey,
            senderCallContractPda: payer.publicKey,
            gatewayRootPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        processError(error, "Call Contract");
    }
  })

  it("CallContractOffchainData", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.callContractOffchainData("1", "2", [1, 2], 3).accounts({
            senderProgram: payer.publicKey,
            senderCallContractPda: payer.publicKey,
            gatewayRootPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        processError(error, "Call Contract Offchain Data");
    }
  })

  it("InitializeConfig", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.initializeConfig(
            [1, 2], [[1], [1, 2]], new BN(3), payer.publicKey, {value: [new BN(2)]}
        ).accounts({
            payer: payer.publicKey,
            upgradeAuthority: payer.publicKey,
            gatewayProgramData: payer.publicKey,
            gatewayConfigPda: payer.publicKey
        }).rpc();
    } catch (error) {
        processError(error, "Initialize Config");
    }
  })

  it("InitializePayloadVerificationSession", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.initializePayloadVerificationSession(
            [1, 2],
        ).accounts({
            payer: payer.publicKey,
            gatewayConfigPda: payer.publicKey,
            verificationSessionPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        processError(error, "Initialize Verification Session");
    }
  })

  it("VerifySignature", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.verifySignature([1], {
            signature: {
                ecdsaRecoverable: {
                    0: [1]
                },
            },
            leaf: {
                nonce: new BN(1),
                quorum: new BN(2),
                signerPubkey: {
                    secp256k1: {
                        0: [2]
                    },
                },
                signerWeight: new BN(3),
                position: 2,
                setSize: 3,
                domainSeparator: [3],
            },
            merkleProof: Buffer.from(new Uint8Array(3)),
        }).accounts({
            gatewayConfigPda: payer.publicKey,
            verificationSessionPda: payer.publicKey,
            verifierSetTrackerPda: payer.publicKey
        }).rpc();
    } catch (error) {
        processError(error, "Verify Signature");
    }
  })

  it("InitializeMessagePayload", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.initializeMessagePayload(
            new BN(1), [2, 3]
        ).accounts({
            payer: payer.publicKey,
            gatewayRootPda: payer.publicKey,
            messagePayloadPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        processError(error, "Initialize Message Payload");
    }
  })

  it("WriteMessagePayload", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.writeMessagePayload(
            new BN(1), Buffer.from(new Uint8Array(2)), [1, 2]
        ).accounts({
            authority: payer.publicKey,
            gatewayRootPda: payer.publicKey,
            messagePayloadPda: payer.publicKey
        }).rpc();
    } catch (error) {
        processError(error, "Write Message Payload");
    }
  })

  it("CommitMessagePayload", async () => {
    const payer = await getKeypairFromFile();
    const [gatewayRootPdaPublicKey, _] = PublicKey.findProgramAddressSync([], payer.publicKey);
    try {
        const tx = await program.methods.commitMessagePayload([1]).accounts({
            authority: payer.publicKey,
            gatewayRootPda: gatewayRootPdaPublicKey,
            messagePayloadPda: gatewayRootPdaPublicKey
        }).rpc();
    } catch (error) {
        processError(error, "Commit Message Payload");
    }
  })

  it("CloseMessagePayload", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.closeMessagePayload(
            [1, 2]
        ).accounts({
            authority: payer.publicKey,
            gatewayRootPda: payer.publicKey,
            messagePayloadPda: payer.publicKey
        }).rpc();
    } catch (error) {
        processError(error, "Close Message Payload");
    }
  })

  it("ValidateMessage", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.validateMessage({
            ccId: {
                chain: "",
                id: "",
            },
            sourceAddress: "",
            destinationChain: "",
            destinationAddress: "",
            payloadHash: [1],
        }).accounts({
            incomingMessagePda: payer.publicKey,
            signingPda: payer.publicKey,
        }).rpc();
    } catch (error) {
        processError(error, "Validate Message");
    }
  })

  it("TransferOperatorship", async () => {
    const payer = await getKeypairFromFile();
    try {
        const tx = await program.methods.transferOperatorship().accounts({
            gatewayRootPda: payer.publicKey,
            currentOperatorOrGatewayProgramOwner: payer.publicKey,
            programdata: payer.publicKey,
            newOperator: payer.publicKey,
        }).rpc();
    } catch (error) {
        processError(error, "Transfer Operatorship");
    }
  })
});