import { Idl, Coder } from "@coral-xyz/anchor";

import { AxelarSolanaItsAccountsCoder } from "./accounts";
import { AxelarSolanaItsEventsCoder } from "./events";
import { AxelarSolanaItsInstructionCoder } from "./instructions";
import { AxelarSolanaItsStateCoder } from "./state";
import { AxelarSolanaItsTypesCoder } from "./types";

/**
 * Coder for AxelarSolanaIts
 */
export class AxelarSolanaItsCoder implements Coder {
  readonly accounts: AxelarSolanaItsAccountsCoder;
  readonly events: AxelarSolanaItsEventsCoder;
  readonly instruction: AxelarSolanaItsInstructionCoder;
  readonly state: AxelarSolanaItsStateCoder;
  readonly types: AxelarSolanaItsTypesCoder;

  constructor(idl: Idl) {
    this.accounts = new AxelarSolanaItsAccountsCoder(idl);
    this.events = new AxelarSolanaItsEventsCoder(idl);
    this.instruction = new AxelarSolanaItsInstructionCoder(idl);
    this.state = new AxelarSolanaItsStateCoder(idl);
    this.types = new AxelarSolanaItsTypesCoder(idl);
  }
}
