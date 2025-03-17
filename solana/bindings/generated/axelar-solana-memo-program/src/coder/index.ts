import { Idl, Coder } from "@coral-xyz/anchor";

import { AxelarSolanaMemoProgramAccountsCoder } from "./accounts";
import { AxelarSolanaMemoProgramEventsCoder } from "./events";
import { AxelarSolanaMemoProgramInstructionCoder } from "./instructions";
import { AxelarSolanaMemoProgramStateCoder } from "./state";
import { AxelarSolanaMemoProgramTypesCoder } from "./types";

/**
 * Coder for AxelarSolanaMemoProgram
 */
export class AxelarSolanaMemoProgramCoder implements Coder {
  readonly accounts: AxelarSolanaMemoProgramAccountsCoder;
  readonly events: AxelarSolanaMemoProgramEventsCoder;
  readonly instruction: AxelarSolanaMemoProgramInstructionCoder;
  readonly state: AxelarSolanaMemoProgramStateCoder;
  readonly types: AxelarSolanaMemoProgramTypesCoder;

  constructor(idl: Idl) {
    this.accounts = new AxelarSolanaMemoProgramAccountsCoder(idl);
    this.events = new AxelarSolanaMemoProgramEventsCoder(idl);
    this.instruction = new AxelarSolanaMemoProgramInstructionCoder(idl);
    this.state = new AxelarSolanaMemoProgramStateCoder(idl);
    this.types = new AxelarSolanaMemoProgramTypesCoder(idl);
  }
}
