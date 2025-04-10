import { AccountMeta, TransactionInstruction } from "@solana/web3.js";
import * as borsh from "@coral-xyz/borsh";
import { arrayify, defaultAbiCoder } from "ethers/lib/utils";

export enum EncodingSchema {
  BORSH = 0,
  ABI = 1,
}

export class SolanaAxelarExecutablePayload {
  readonly encodingSchema: EncodingSchema;
  readonly executePayload: Buffer;
  readonly accounts: AccountMeta[];

  constructor(instruction: TransactionInstruction, encoding: EncodingSchema) {
    this.encodingSchema = encoding;
    this.executePayload = instruction.data;
    this.accounts = instruction.keys;
  }

  encode(): Buffer {
    if (this.encodingSchema === EncodingSchema.ABI) {
      return this.abiEncode();
    } else {
      return this.borshEncode();
    }
  }

  private borshEncode(): Buffer {
    let accountsSpan = 4 + 33 * this.accounts.length;
    let executableSpan = 4 + this.executePayload.length;

    let accounts = this.accounts.map((account) => {
      let flags = 0;

      if (account.isSigner) flags |= 1;
      if (account.isWritable) flags |= 2;

      return { pubkey: account.pubkey, flags };
    });

    let buffer = Buffer.alloc(accountsSpan + executableSpan + 5);

    buffer.writeUint8(EncodingSchema.BORSH);

    let bytes = ExecutablePayloadLayout.encode(
      { executePayload: this.executePayload, accounts },
      buffer,
      1
    );

    return buffer.slice(0, bytes + 1);
  }

  private abiEncode(): Buffer {
    let accounts = this.accounts.map((account) => ({
      pubkey: account.pubkey.toBytes(),
      isSigner: account.isSigner,
      isWritable: account.isWritable,
    }));

    let encodedPayload = defaultAbiCoder.encode(
      ["bytes", "tuple(bytes32 pubkey, bool isSigner, bool isWritable)[]"],
      [this.executePayload, accounts]
    );
    let buffer = Buffer.alloc(1);
    buffer.writeUint8(EncodingSchema.ABI);

    return Buffer.from([...buffer, ...arrayify(encodedPayload)]);
  }
}

const AccountMetaLayout = borsh.struct([
  borsh.publicKey("pubkey"),
  borsh.u8("flags"),
]);

const ExecutablePayloadLayout = borsh.struct([
  borsh.vecU8("executePayload"),
  borsh.vec(AccountMetaLayout, "accounts"),
]);
