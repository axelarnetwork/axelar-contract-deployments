import { PublicKey } from "@solana/web3.js";
import { keccak256, arrayify } from "ethers/lib/utils";
import { AXELAR_SOLANA_ITS_PROGRAM_ID as PROGRAM_ID } from "../../generated/axelar-solana-its/src";

// Seed prefixes - these must match the Rust implementation
const ITS_SEED = "interchain-token-service";
const TOKEN_MANAGER_SEED = "token-manager";
const INTERCHAIN_TOKEN_SEED = "interchain-token";
const PREFIX_INTERCHAIN_TOKEN_ID = "interchain-token-id";
const PREFIX_INTERCHAIN_TOKEN_SALT = "interchain-token-salt";
const PREFIX_CANONICAL_TOKEN_SALT = "canonical-token-salt";
const PREFIX_CUSTOM_TOKEN_SALT = "solana-custom-token-salt";
const FLOW_SLOT_SEED = "flow-slot";
const DEPLOYMENT_APPROVAL_SEED = "deployment-approval";
const USER_ROLES_SEED = "user-roles";
const ROLE_RPOPOSAL_SEED = "role-proposal";
const CALL_CONTRACT_SIGNING_SEED = "gtw-call-contract";

export const TOKEN_METADATA_PROGRAM_ID = new PublicKey(
  "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);

// PDA derivation functions
export function findItsRootPda(
  gatewayRootPda: PublicKey
): [publicKey: PublicKey, bump: number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(ITS_SEED), gatewayRootPda.toBuffer()],
    PROGRAM_ID
  );
}

export function createItsRootPda(
  gatewayRootPda: PublicKey,
  bump: number
): PublicKey {
  return PublicKey.createProgramAddressSync(
    [Buffer.from(ITS_SEED), gatewayRootPda.toBuffer(), Buffer.from([bump])],
    PROGRAM_ID
  );
}

export function findTokenManagerPda(
  itsRootPda: PublicKey,
  tokenId: Uint8Array
): [publicKey: PublicKey, bump: number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(TOKEN_MANAGER_SEED), itsRootPda.toBuffer(), tokenId],
    PROGRAM_ID
  );
}

export function createTokenManagerPda(
  itsRootPda: PublicKey,
  tokenId: Uint8Array,
  bump: number
): PublicKey {
  return PublicKey.createProgramAddressSync(
    [
      Buffer.from(TOKEN_MANAGER_SEED),
      itsRootPda.toBuffer(),
      tokenId,
      Buffer.from([bump]),
    ],
    PROGRAM_ID
  );
}

export function findInterchainTokenPda(
  itsRootPda: PublicKey,
  tokenId: Uint8Array
): [publicKey: PublicKey, bump: number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(INTERCHAIN_TOKEN_SEED), itsRootPda.toBuffer(), tokenId],
    PROGRAM_ID
  );
}

export function createInterchainTokenPda(
  itsRootPda: PublicKey,
  tokenId: Uint8Array,
  bump: number
): PublicKey {
  return PublicKey.createProgramAddressSync(
    [
      Buffer.from(INTERCHAIN_TOKEN_SEED),
      itsRootPda.toBuffer(),
      tokenId,
      Buffer.from([bump]),
    ],
    PROGRAM_ID
  );
}

export function findFlowSlotPda(
  tokenManagerPda: PublicKey,
  epoch: number
): [publicKey: PublicKey, bump: number] {
  const epochBuffer = Buffer.alloc(8);
  epochBuffer.writeBigUInt64LE(BigInt(epoch));
  return PublicKey.findProgramAddressSync(
    [Buffer.from(FLOW_SLOT_SEED), tokenManagerPda.toBuffer(), epochBuffer],
    PROGRAM_ID
  );
}

export function createFlowSlotPda(
  tokenManagerPda: PublicKey,
  epoch: number,
  bump: number
): PublicKey {
  const epochBuffer = Buffer.alloc(8);
  epochBuffer.writeBigUInt64LE(BigInt(epoch));
  return PublicKey.createProgramAddressSync(
    [
      Buffer.from(FLOW_SLOT_SEED),
      tokenManagerPda.toBuffer(),
      epochBuffer,
      Buffer.from([bump]),
    ],
    PROGRAM_ID
  );
}

export function findDeploymentApprovalPda(
  minter: PublicKey,
  tokenId: Uint8Array,
  destinationChain: string
): [publicKey: PublicKey, bump: number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(DEPLOYMENT_APPROVAL_SEED),
      minter.toBuffer(),
      tokenId,
      Buffer.from(destinationChain),
    ],
    PROGRAM_ID
  );
}

export function createDeploymentApprovalPda(
  minter: PublicKey,
  tokenId: Uint8Array,
  destinationChain: string,
  bump: number
): PublicKey {
  return PublicKey.createProgramAddressSync(
    [
      Buffer.from(DEPLOYMENT_APPROVAL_SEED),
      minter.toBuffer(),
      tokenId,
      Buffer.from(destinationChain),
      Buffer.from([bump]),
    ],
    PROGRAM_ID
  );
}

// Helper functions for PDA validation
export function validateItsRootPda(
  pda: PublicKey,
  gatewayRootPda: PublicKey,
  bump: number
): boolean {
  const expectedPda = createItsRootPda(gatewayRootPda, bump);
  return expectedPda.equals(pda);
}

export function validateTokenManagerPda(
  pda: PublicKey,
  itsRootPda: PublicKey,
  tokenId: Uint8Array,
  bump: number
): boolean {
  const expectedPda = createTokenManagerPda(itsRootPda, tokenId, bump);
  return expectedPda.equals(pda);
}

export function validateInterchainTokenPda(
  pda: PublicKey,
  itsRootPda: PublicKey,
  tokenId: Uint8Array,
  bump: number
): boolean {
  const expectedPda = createInterchainTokenPda(itsRootPda, tokenId, bump);
  return expectedPda.equals(pda);
}

export function validateFlowSlotPda(
  pda: PublicKey,
  tokenManagerPda: PublicKey,
  epoch: number,
  bump: number
): boolean {
  const expectedPda = createFlowSlotPda(tokenManagerPda, epoch, bump);
  return expectedPda.equals(pda);
}

export function validateDeploymentApprovalPda(
  pda: PublicKey,
  minter: PublicKey,
  tokenId: Uint8Array,
  destinationChain: string,
  bump: number
): boolean {
  const expectedPda = createDeploymentApprovalPda(
    minter,
    tokenId,
    destinationChain,
    bump
  );
  return expectedPda.equals(pda);
}

// Hash functions matching Rust implementation
export function canonicalInterchainTokenDeploySalt(
  mint: PublicKey
): Uint8Array {
  return arrayify(
    keccak256(
      new Uint8Array([
        ...Buffer.from(PREFIX_CANONICAL_TOKEN_SALT),
        ...mint.toBytes(),
      ])
    )
  );
}

export function interchainTokenDeployerSalt(
  deployer: PublicKey,
  salt: Uint8Array
): Uint8Array {
  return arrayify(
    keccak256(
      new Uint8Array([
        ...Buffer.from(PREFIX_INTERCHAIN_TOKEN_SALT),
        ...deployer.toBytes(),
        ...salt,
      ])
    )
  );
}

export function linkedTokenDeployerSalt(
  deployer: PublicKey,
  salt: Uint8Array
): Uint8Array {
  return arrayify(
    keccak256(
      new Uint8Array([
        ...Buffer.from(PREFIX_CUSTOM_TOKEN_SALT),
        ...deployer.toBytes(),
        ...salt,
      ])
    )
  );
}

export function interchainTokenIdInternal(salt: Uint8Array): Uint8Array {
  return arrayify(
    keccak256(
      new Uint8Array([...Buffer.from(PREFIX_INTERCHAIN_TOKEN_ID), ...salt])
    )
  );
}

export function interchainTokenId(
  deployer: PublicKey,
  salt: Uint8Array
): Uint8Array {
  const deploySalt = interchainTokenDeployerSalt(deployer, salt);
  return interchainTokenIdInternal(deploySalt);
}

export function canonicalInterchainTokenId(mint: PublicKey): Uint8Array {
  const salt = canonicalInterchainTokenDeploySalt(mint);
  return interchainTokenIdInternal(salt);
}

export function linkedTokenId(
  deployer: PublicKey,
  salt: Uint8Array
): Uint8Array {
  const linkedTokenSalt = linkedTokenDeployerSalt(deployer, salt);
  return interchainTokenIdInternal(linkedTokenSalt);
}

// Role Management PDAs
export function findUserRolesPda(
  resource: PublicKey,
  user: PublicKey
): [publicKey: PublicKey, bump: number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(USER_ROLES_SEED), resource.toBuffer(), user.toBuffer()],
    PROGRAM_ID
  );
}

export function findProposalPda(
  resource: PublicKey,
  from: PublicKey,
  to: PublicKey
): [publicKey: PublicKey, bump: number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(ROLE_RPOPOSAL_SEED),
      resource.toBuffer(),
      from.toBuffer(),
      to.toBuffer(),
    ],
    PROGRAM_ID
  );
}

export function findMetadataPda(mint: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("metadata"),
      TOKEN_METADATA_PROGRAM_ID.toBuffer(),
      mint.toBuffer(),
    ],
    TOKEN_METADATA_PROGRAM_ID
  );
}

export function findCallContractSigningPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(CALL_CONTRACT_SIGNING_SEED)],
    PROGRAM_ID
  );
}
