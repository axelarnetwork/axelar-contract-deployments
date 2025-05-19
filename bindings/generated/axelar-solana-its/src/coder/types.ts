import { Idl, TypesCoder } from "@coral-xyz/anchor";

export class AxelarSolanaItsTypesCoder implements TypesCoder {
  constructor(_idl: Idl) {}

  encode<T = any>(_name: string, _type: T): Buffer {
    throw new Error("AxelarSolanaIts does not have user-defined types");
  }
  decode<T = any>(_name: string, _typeData: Buffer): T {
    throw new Error("AxelarSolanaIts does not have user-defined types");
  }
}
