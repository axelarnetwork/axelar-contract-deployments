// @ts-nocheck
import * as B from "@native-to-anchor/buffer-layout";
import { Idl, InstructionCoder } from "@coral-xyz/anchor";

export class AxelarSolanaMemoProgramInstructionCoder
  implements InstructionCoder
{
  constructor(_idl: Idl) {}

  encode(ixName: string, ix: any): Buffer {
    switch (ixName) {
      case "initialize": {
        return encodeInitialize(ix);
      }
      case "processMemo": {
        return encodeProcessMemo(ix);
      }
      case "sendToGateway": {
        return encodeSendToGateway(ix);
      }
      case "sendToGatewayOffchainMemo": {
        return encodeSendToGatewayOffchainMemo(ix);
      }

      default: {
        throw new Error(`Invalid instruction: ${ixName}`);
      }
    }
  }

  encodeState(_ixName: string, _ix: any): Buffer {
    throw new Error("AxelarSolanaMemoProgram does not have state");
  }
}

function encodeInitialize({ counterPdaBump }: any): Buffer {
  return encodeData({ initialize: { counterPdaBump } }, 1 + 1);
}

function encodeProcessMemo({ memo }: any): Buffer {
  return encodeData({ processMemo: { memo } }, 1 + 4 + memo.length);
}

function encodeSendToGateway({
  memo,
  destinationChain,
  destinationAddress,
}: any): Buffer {
  return encodeData(
    { sendToGateway: { memo, destinationChain, destinationAddress } },
    1 +
      4 +
      memo.length +
      4 +
      destinationChain.length +
      4 +
      destinationAddress.length
  );
}

function encodeSendToGatewayOffchainMemo({
  memoHash,
  destinationChain,
  destinationAddress,
}: any): Buffer {
  return encodeData(
    {
      sendToGatewayOffchainMemo: {
        memoHash,
        destinationChain,
        destinationAddress,
      },
    },
    1 + 1 * 32 + 4 + destinationChain.length + 4 + destinationAddress.length
  );
}

const LAYOUT = B.union(B.u8("instruction"));
LAYOUT.addVariant(0, B.struct([B.u8("counterPdaBump")]), "initialize");
LAYOUT.addVariant(1, B.struct([B.utf8Str("memo")]), "processMemo");
LAYOUT.addVariant(
  2,
  B.struct([
    B.utf8Str("memo"),
    B.utf8Str("destinationChain"),
    B.utf8Str("destinationAddress"),
  ]),
  "sendToGateway"
);
LAYOUT.addVariant(
  3,
  B.struct([
    B.seq(B.u8(), 32, "memoHash"),
    B.utf8Str("destinationChain"),
    B.utf8Str("destinationAddress"),
  ]),
  "sendToGatewayOffchainMemo"
);

function encodeData(ix: any, span: number): Buffer {
  const b = Buffer.alloc(span);
  LAYOUT.encode(ix, b);
  return b;
}
