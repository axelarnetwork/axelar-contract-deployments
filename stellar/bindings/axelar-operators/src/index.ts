import { Buffer } from "buffer";
import { Address } from '@stellar/stellar-sdk';
import {
  AssembledTransaction,
  Client as ContractClient,
  ClientOptions as ContractClientOptions,
  Result,
  Spec as ContractSpec,
} from '@stellar/stellar-sdk/contract';
import type {
  u32,
  i32,
  u64,
  i64,
  u128,
  i128,
  u256,
  i256,
  Option,
  Typepoint,
  Duration,
} from '@stellar/stellar-sdk/contract';
export * from '@stellar/stellar-sdk'
export * as contract from '@stellar/stellar-sdk/contract'
export * as rpc from '@stellar/stellar-sdk/rpc'

if (typeof window !== 'undefined') {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
}


export const networks = {
  testnet: {
    networkPassphrase: "Test SDF Network ; September 2015",
    contractId: "CAXSQQF3TLPKL4L4K7WRQNLG4K4FNKFTMHPZJKZEQ7KJCHX2VX4GD3MT",
  }
} as const

export const Errors = {
  1: {message:""},
  2: {message:""}
}
export type DataKey = {tag: "Initialized", values: void} | {tag: "Owner", values: void} | {tag: "Operators", values: readonly [string]};


export interface WeightedSigner {
  signer: Buffer;
  weight: u256;
}


export interface WeightedSigners {
  nonce: Buffer;
  signers: Array<WeightedSigner>;
  threshold: u256;
}


export interface Proof {
  signatures: Array<readonly [Buffer, u32]>;
  signers: WeightedSigners;
}


export interface Message {
  contract_address: string;
  message_id: string;
  payload_hash: Buffer;
  source_address: string;
  source_chain: string;
}


export interface Token {
  address: string;
  amount: i128;
}


export interface Client {
  /**
   * Construct and simulate a transfer_ownership transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  transfer_ownership: ({new_owner}: {new_owner: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a owner transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  owner: (options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  initialize: ({owner}: {owner: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a is_operator transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_operator: ({account}: {account: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a add_operator transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  add_operator: ({account}: {account: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a remove_operator transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  remove_operator: ({account}: {account: string}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a execute transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  execute: ({operator, contract, func, args}: {operator: string, contract: string, func: string, args: Array<any>}, options?: {
    /**
     * The fee to pay for the transaction. Default: BASE_FEE
     */
    fee?: number;

    /**
     * The maximum amount of time to wait for the transaction to complete. Default: DEFAULT_TIMEOUT
     */
    timeoutInSeconds?: number;

    /**
     * Whether to automatically simulate the transaction when constructing the AssembledTransaction. Default: true
     */
    simulate?: boolean;
  }) => Promise<AssembledTransaction<any>>

}
export class Client extends ContractClient {
  constructor(public readonly options: ContractClientOptions) {
    super(
      new ContractSpec([ "AAAAAAAAAAAAAAASdHJhbnNmZXJfb3duZXJzaGlwAAAAAAABAAAAAAAAAAluZXdfb3duZXIAAAAAAAATAAAAAA==",
        "AAAAAAAAAAAAAAAFb3duZXIAAAAAAAAAAAAAAQAAABM=",
        "AAAAAAAAAAAAAAAKaW5pdGlhbGl6ZQAAAAAAAQAAAAAAAAAFb3duZXIAAAAAAAATAAAAAA==",
        "AAAAAAAAAAAAAAALaXNfb3BlcmF0b3IAAAAAAQAAAAAAAAAHYWNjb3VudAAAAAATAAAAAQAAAAE=",
        "AAAAAAAAAAAAAAAMYWRkX29wZXJhdG9yAAAAAQAAAAAAAAAHYWNjb3VudAAAAAATAAAAAA==",
        "AAAAAAAAAAAAAAAPcmVtb3ZlX29wZXJhdG9yAAAAAAEAAAAAAAAAB2FjY291bnQAAAAAEwAAAAA=",
        "AAAAAAAAAAAAAAAHZXhlY3V0ZQAAAAAEAAAAAAAAAAhvcGVyYXRvcgAAABMAAAAAAAAACGNvbnRyYWN0AAAAEwAAAAAAAAAEZnVuYwAAABEAAAAAAAAABGFyZ3MAAAPqAAAAAAAAAAEAAAAA",
        "AAAABAAAAAAAAAAAAAAABUVycm9yAAAAAAAAAgAAAAAAAAAUT3BlcmF0b3JBbHJlYWR5QWRkZWQAAAABAAAAAAAAAA1Ob3RBbk9wZXJhdG9yAAAAAAAAAg==",
        "AAAAAgAAAAAAAAAAAAAAB0RhdGFLZXkAAAAAAwAAAAAAAAAAAAAAC0luaXRpYWxpemVkAAAAAAAAAAAAAAAABU93bmVyAAAAAAAAAQAAAAAAAAAJT3BlcmF0b3JzAAAAAAAAAQAAABM=",
        "AAAAAQAAAAAAAAAAAAAADldlaWdodGVkU2lnbmVyAAAAAAACAAAAAAAAAAZzaWduZXIAAAAAA+4AAAAgAAAAAAAAAAZ3ZWlnaHQAAAAAAAw=",
        "AAAAAQAAAAAAAAAAAAAAD1dlaWdodGVkU2lnbmVycwAAAAADAAAAAAAAAAVub25jZQAAAAAAA+4AAAAgAAAAAAAAAAdzaWduZXJzAAAAA+oAAAfQAAAADldlaWdodGVkU2lnbmVyAAAAAAAAAAAACXRocmVzaG9sZAAAAAAAAAw=",
        "AAAAAQAAAAAAAAAAAAAABVByb29mAAAAAAAAAgAAAAAAAAAKc2lnbmF0dXJlcwAAAAAD6gAAA+0AAAACAAAD7gAAAEAAAAAEAAAAAAAAAAdzaWduZXJzAAAAB9AAAAAPV2VpZ2h0ZWRTaWduZXJzAA==",
        "AAAAAQAAAAAAAAAAAAAAB01lc3NhZ2UAAAAABQAAAAAAAAAQY29udHJhY3RfYWRkcmVzcwAAABMAAAAAAAAACm1lc3NhZ2VfaWQAAAAAABAAAAAAAAAADHBheWxvYWRfaGFzaAAAA+4AAAAgAAAAAAAAAA5zb3VyY2VfYWRkcmVzcwAAAAAAEAAAAAAAAAAMc291cmNlX2NoYWluAAAAEA==",
        "AAAAAQAAAAAAAAAAAAAABVRva2VuAAAAAAAAAgAAAAAAAAAHYWRkcmVzcwAAAAATAAAAAAAAAAZhbW91bnQAAAAAAAs=" ]),
      options
    )
  }
  public readonly fromJSON = {
    transfer_ownership: this.txFromJSON<null>,
        owner: this.txFromJSON<string>,
        initialize: this.txFromJSON<null>,
        is_operator: this.txFromJSON<boolean>,
        add_operator: this.txFromJSON<null>,
        remove_operator: this.txFromJSON<null>,
        execute: this.txFromJSON<any>
  }
}