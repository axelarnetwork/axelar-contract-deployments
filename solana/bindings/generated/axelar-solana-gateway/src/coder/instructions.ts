// @ts-nocheck
import * as B from "@native-to-anchor/buffer-layout";
import { Idl, InstructionCoder } from "@coral-xyz/anchor";

export class AxelarSolanaGatewayInstructionCoder implements InstructionCoder {
  constructor(_idl: Idl) {}

  encode(ixName: string, ix: any): Buffer {
    switch (ixName) {
      case "approveMessage": {
        return encodeApproveMessage(ix);
      }
      case "rotateSigners": {
        return encodeRotateSigners(ix);
      }
      case "callContract": {
        return encodeCallContract(ix);
      }
      case "callContractOffchainData": {
        return encodeCallContractOffchainData(ix);
      }
      case "initializeConfig": {
        return encodeInitializeConfig(ix);
      }
      case "initializePayloadVerificationSession": {
        return encodeInitializePayloadVerificationSession(ix);
      }
      case "verifySignature": {
        return encodeVerifySignature(ix);
      }
      case "initializeMessagePayload": {
        return encodeInitializeMessagePayload(ix);
      }
      case "writeMessagePayload": {
        return encodeWriteMessagePayload(ix);
      }
      case "commitMessagePayload": {
        return encodeCommitMessagePayload(ix);
      }
      case "closeMessagePayload": {
        return encodeCloseMessagePayload(ix);
      }
      case "validateMessage": {
        return encodeValidateMessage(ix);
      }
      case "transferOperatorship": {
        return encodeTransferOperatorship(ix);
      }

      default: {
        throw new Error(`Invalid instruction: ${ixName}`);
      }
    }
  }

  encodeState(_ixName: string, _ix: any): Buffer {
    throw new Error("AxelarSolanaGateway does not have state");
  }
}

function encodeApproveMessage({ message, payloadMerkleRoot }: any): Buffer {
  return encodeData(
    { approveMessage: { message, payloadMerkleRoot } },
    1 +
      4 +
      message.leaf.message.ccId.chain.length +
      4 +
      message.leaf.message.ccId.id.length +
      4 +
      message.leaf.message.sourceAddress.length +
      4 +
      message.leaf.message.destinationChain.length +
      4 +
      message.leaf.message.destinationAddress.length +
      1 * 32 +
      2 +
      2 +
      1 * 32 +
      1 * 32 +
      4 +
      message.proof.length +
      1 * 32
  );
}

function encodeRotateSigners({ newVerifierSetMerkleRoot }: any): Buffer {
  return encodeData(
    { rotateSigners: { newVerifierSetMerkleRoot } },
    1 + 1 * 32
  );
}

function encodeCallContract({
  destinationChain,
  destinationContractAddress,
  payload,
  signingPdaBump,
}: any): Buffer {
  return encodeData(
    {
      callContract: {
        destinationChain,
        destinationContractAddress,
        payload,
        signingPdaBump,
      },
    },
    1 +
      4 +
      destinationChain.length +
      4 +
      destinationContractAddress.length +
      4 +
      payload.length +
      1
  );
}

function encodeCallContractOffchainData({
  destinationChain,
  destinationContractAddress,
  payloadHash,
  signingPdaBump,
}: any): Buffer {
  return encodeData(
    {
      callContractOffchainData: {
        destinationChain,
        destinationContractAddress,
        payloadHash,
        signingPdaBump,
      },
    },
    1 +
      4 +
      destinationChain.length +
      4 +
      destinationContractAddress.length +
      1 * 32 +
      1
  );
}

function encodeInitializeConfig({
  domainSeparator,
  initialSignerSets,
  minimumRotationDelay,
  operator,
  previousVerifierRetention,
}: any): Buffer {
  return encodeData(
    {
      initializeConfig: {
        domainSeparator,
        initialSignerSets,
        minimumRotationDelay,
        operator,
        previousVerifierRetention,
      },
    },
    1 + 1 * 32 + 4 + initialSignerSets.length * 32 + 8 + 32 + 8 * 4
  );
}

function encodeInitializePayloadVerificationSession({
  payloadMerkleRoot,
}: any): Buffer {
  return encodeData(
    { initializePayloadVerificationSession: { payloadMerkleRoot } },
    1 + 1 * 32
  );
}

function encodeVerifySignature({
  payloadMerkleRoot,
  verifierInfo,
}: any): Buffer {
  const signatureKey = Object.keys(verifierInfo.signature)[0];
  const signatureValue = verifierInfo.signature[signatureKey];
  verifierInfo.signature[signatureKey] = signatureValue['0'];

  const signerPubKey = Object.keys(verifierInfo.leaf.signerPubkey)[0];
  const signerPubKeyValue = verifierInfo.leaf.signerPubkey[signerPubKey];
  verifierInfo.leaf.signerPubkey[signerPubKey] = signerPubKeyValue['0'];

  return encodeData(
    { verifySignature: { payloadMerkleRoot, verifierInfo } },
    1 +
      1 * 32 +
      (() => {
        switch (Object.keys(verifierInfo.signature)[0]) {
          case "ecdsaRecoverable":
            return 1 + 1 * 65;
          case "ed25519":
            return 1 + 1 * 64;
        }
      })() +
      8 +
      16 +
      (() => {
        switch (Object.keys(verifierInfo.leaf.signerPubkey)[0]) {
          case "secp256k1":
            return 1 + 1 * 33;
          case "ed25519":
            return 1 + 1 * 32;
        }
      })() +
      16 +
      2 +
      2 +
      1 * 32 +
      4 +
      verifierInfo.merkleProof.length
  );
}

function encodeInitializeMessagePayload({
  bufferSize,
  commandId,
}: any): Buffer {
  return encodeData(
    { initializeMessagePayload: { bufferSize, commandId } },
    1 + 8 + 1 * 32
  );
}

function encodeWriteMessagePayload({ offset, bytes, commandId }: any): Buffer {
  return encodeData(
    { writeMessagePayload: { offset, bytes, commandId } },
    1 + 8 + 4 + bytes.length + 1 * 32
  );
}

function encodeCommitMessagePayload({ commandId }: any): Buffer {
  return encodeData({ commitMessagePayload: { commandId } }, 1 + 1 * 32);
}

function encodeCloseMessagePayload({ commandId }: any): Buffer {
  return encodeData({ closeMessagePayload: { commandId } }, 1 + 1 * 32);
}

function encodeValidateMessage({ message }: any): Buffer {
  return encodeData(
    { validateMessage: { message } },
    1 +
      4 +
      message.ccId.chain.length +
      4 +
      message.ccId.id.length +
      4 +
      message.sourceAddress.length +
      4 +
      message.destinationChain.length +
      4 +
      message.destinationAddress.length +
      1 * 32
  );
}

function encodeTransferOperatorship({}: any): Buffer {
  return encodeData({ transferOperatorship: {} }, 1);
}

const LAYOUT = B.union(B.u8("instruction"));
LAYOUT.addVariant(
  0,
  B.struct([
    B.struct(
      [
        B.struct(
          [
            B.struct(
              [
                B.struct([B.utf8Str("chain"), B.utf8Str("id")], "ccId"),
                B.utf8Str("sourceAddress"),
                B.utf8Str("destinationChain"),
                B.utf8Str("destinationAddress"),
                B.seq(B.u8(), 32, "payloadHash"),
              ],
              "message"
            ),
            B.u16("position"),
            B.u16("setSize"),
            B.seq(B.u8(), 32, "domainSeparator"),
            B.seq(B.u8(), 32, "signingVerifierSet"),
          ],
          "leaf"
        ),
        B.bytes("proof"),
      ],
      "message"
    ),
    B.seq(B.u8(), 32, "payloadMerkleRoot"),
  ]),
  "approveMessage"
);
LAYOUT.addVariant(
  1,
  B.struct([B.seq(B.u8(), 32, "newVerifierSetMerkleRoot")]),
  "rotateSigners"
);
LAYOUT.addVariant(
  2,
  B.struct([
    B.utf8Str("destinationChain"),
    B.utf8Str("destinationContractAddress"),
    B.bytes("payload"),
    B.u8("signingPdaBump"),
  ]),
  "callContract"
);
LAYOUT.addVariant(
  3,
  B.struct([
    B.utf8Str("destinationChain"),
    B.utf8Str("destinationContractAddress"),
    B.seq(B.u8(), 32, "payloadHash"),
    B.u8("signingPdaBump"),
  ]),
  "callContractOffchainData"
);
LAYOUT.addVariant(
  4,
  B.struct([
    B.seq(B.u8(), 32, "domainSeparator"),
    B.vec(B.seq(B.u8(), 32), "initialSignerSets"),
    B.u64("minimumRotationDelay"),
    B.publicKey("operator"),
    B.struct([B.seq(B.u64(), 4, "value")], "previousVerifierRetention"),
  ]),
  "initializeConfig"
);
LAYOUT.addVariant(
  5,
  B.struct([B.seq(B.u8(), 32, "payloadMerkleRoot")]),
  "initializePayloadVerificationSession"
);
LAYOUT.addVariant(
  6,
  B.struct([
    B.seq(B.u8(), 32, "payloadMerkleRoot"),
    B.struct(
      [
        ((p: string) => {
          const U = B.union(B.u8("discriminator"), null, p);
          U.addVariant(0, B.seq(B.u8(), 65), "ecdsaRecoverable");
          U.addVariant(1, B.seq(B.u8(), 64), "ed25519");
          return U;
        })("signature"),
        B.struct(
          [
            B.u64("nonce"),
            B.u128("quorum"),
            ((p: string) => {
              const U = B.union(B.u8("discriminator"), null, p);
              U.addVariant(0, B.seq(B.u8(), 33), "secp256k1");
              U.addVariant(1, B.seq(B.u8(), 32), "ed25519");
              return U;
            })("signerPubkey"),
            B.u128("signerWeight"),
            B.u16("position"),
            B.u16("setSize"),
            B.seq(B.u8(), 32, "domainSeparator"),
          ],
          "leaf"
        ),
        B.bytes("merkleProof"),
      ],
      "verifierInfo"
    ),
  ]),
  "verifySignature"
);
LAYOUT.addVariant(
  7,
  B.struct([B.u64("bufferSize"), B.seq(B.u8(), 32, "commandId")]),
  "initializeMessagePayload"
);
LAYOUT.addVariant(
  8,
  B.struct([B.u64("offset"), B.bytes("bytes"), B.seq(B.u8(), 32, "commandId")]),
  "writeMessagePayload"
);
LAYOUT.addVariant(
  9,
  B.struct([B.seq(B.u8(), 32, "commandId")]),
  "commitMessagePayload"
);
LAYOUT.addVariant(
  10,
  B.struct([B.seq(B.u8(), 32, "commandId")]),
  "closeMessagePayload"
);
LAYOUT.addVariant(
  11,
  B.struct([
    B.struct(
      [
        B.struct([B.utf8Str("chain"), B.utf8Str("id")], "ccId"),
        B.utf8Str("sourceAddress"),
        B.utf8Str("destinationChain"),
        B.utf8Str("destinationAddress"),
        B.seq(B.u8(), 32, "payloadHash"),
      ],
      "message"
    ),
  ]),
  "validateMessage"
);
LAYOUT.addVariant(12, B.struct([]), "transferOperatorship");

function encodeData(ix: any, span: number): Buffer {
  const b = Buffer.alloc(span);
  LAYOUT.encode(ix, b);
  return b;
}
