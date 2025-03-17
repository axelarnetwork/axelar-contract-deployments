import { Idl, Coder } from "@coral-xyz/anchor";

import { AxelarSolanaGatewayAccountsCoder } from "./accounts";
import { AxelarSolanaGatewayEventsCoder } from "./events";
import { AxelarSolanaGatewayInstructionCoder } from "./instructions";
import { AxelarSolanaGatewayStateCoder } from "./state";
import { AxelarSolanaGatewayTypesCoder } from "./types";

/**
 * Coder for AxelarSolanaGateway
 */
export class AxelarSolanaGatewayCoder implements Coder {
  readonly accounts: AxelarSolanaGatewayAccountsCoder;
  readonly events: AxelarSolanaGatewayEventsCoder;
  readonly instruction: AxelarSolanaGatewayInstructionCoder;
  readonly state: AxelarSolanaGatewayStateCoder;
  readonly types: AxelarSolanaGatewayTypesCoder;

  constructor(idl: Idl) {
    this.accounts = new AxelarSolanaGatewayAccountsCoder(idl);
    this.events = new AxelarSolanaGatewayEventsCoder(idl);
    this.instruction = new AxelarSolanaGatewayInstructionCoder(idl);
    this.state = new AxelarSolanaGatewayStateCoder(idl);
    this.types = new AxelarSolanaGatewayTypesCoder(idl);
  }
}
