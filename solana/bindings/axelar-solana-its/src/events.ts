// event-definitions.ts
import { PublicKey } from "@solana/web3.js";
import { Buffer } from "buffer";
import {
  calculateDiscriminant,
  discToHex,
  readPubkey,
  readFixedU8Array,
  readString,
  readVecU8,
  readU8,
  readU64LE,
  MissingDataError,
  TrailingSegmentsError,
  BaseEvent,
  EventClassType,
  EventParserMap,
} from "../../event-utils/src";

// Helper type for constructor fields
type InterchainTransferFields = {
  tokenId: Buffer;
  sourceAddress: PublicKey;
  destinationChain: string;
  destinationAddress: Buffer;
  amount: number;
  dataHash: Buffer;
};

export class InterchainTransfer extends BaseEvent {
  static override readonly EVENT_NAME = "InterchainTransfer";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);

  readonly tokenId: Buffer;
  readonly sourceAddress: PublicKey;
  readonly destinationChain: string;
  readonly destinationAddress: Buffer;
  readonly amount: number;
  readonly dataHash: Buffer;

  constructor(fields: InterchainTransferFields) {
    super();
    this.tokenId = fields.tokenId;
    this.sourceAddress = fields.sourceAddress;
    this.destinationChain = fields.destinationChain;
    this.destinationAddress = fields.destinationAddress;
    this.amount = fields.amount;
    this.dataHash = fields.dataHash;
  }

  static override deserialize(segments: Buffer[]): InterchainTransfer {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };

    const fields: InterchainTransferFields = {
      tokenId: readFixedU8Array(next("tokenId"), 32, "tokenId"),
      sourceAddress: readPubkey(next("sourceAddress"), "sourceAddress"),
      destinationChain: readString(
        next("destinationChain"),
        "destinationChain"
      ),
      destinationAddress: readVecU8(
        next("destinationAddress"),
        "destinationAddress"
      ),
      amount: readU64LE(next("amount"), "amount"),
      dataHash: readFixedU8Array(next("dataHash"), 32, "dataHash"),
    };

    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new InterchainTransfer(fields);
  }
}

type InterchainTransferReceivedFields = {
  commandId: Buffer;
  tokenId: Buffer;
  sourceChain: string;
  sourceAddress: Buffer;
  destinationAddress: PublicKey;
  amount: number;
  dataHash: Buffer;
};

export class InterchainTransferReceived extends BaseEvent {
  static override readonly EVENT_NAME = "InterchainTransferReceived";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly commandId: Buffer;
  readonly tokenId: Buffer;
  readonly sourceChain: string;
  readonly sourceAddress: Buffer;
  readonly destinationAddress: PublicKey;
  readonly amount: number;
  readonly dataHash: Buffer;

  constructor(fields: InterchainTransferReceivedFields) {
    super();

    this.commandId = fields.commandId;
    this.tokenId = fields.tokenId;
    this.sourceChain = fields.sourceChain;
    this.sourceAddress = fields.sourceAddress;
    this.destinationAddress = fields.destinationAddress;
    this.amount = fields.amount;
    this.dataHash = fields.dataHash;
  }

  static override deserialize(segments: Buffer[]): InterchainTransferReceived {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: InterchainTransferReceivedFields = {
      commandId: readFixedU8Array(next("commandId"), 32, "commandId"),
      tokenId: readFixedU8Array(next("tokenId"), 32, "tokenId"),
      sourceChain: readString(next("sourceChain"), "sourceChain"),
      sourceAddress: readVecU8(next("sourceAddress"), "sourceAddress"),
      destinationAddress: readPubkey(
        next("destinationAddress"),
        "destinationAddress"
      ),
      amount: readU64LE(next("amount"), "amount"),
      dataHash: readFixedU8Array(next("dataHash"), 32, "dataHash"),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new InterchainTransferReceived(fields);
  }
}

type TokenMetadataRegisteredFields = {
  tokenAddress: PublicKey;
  decimals: number;
};

export class TokenMetadataRegistered extends BaseEvent {
  static override readonly EVENT_NAME = "TokenMetadataRegistered";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly tokenAddress: PublicKey;
  readonly decimals: number;

  constructor(fields: TokenMetadataRegisteredFields) {
    super();

    this.tokenAddress = fields.tokenAddress;
    this.decimals = fields.decimals;
  }
  static override deserialize(segments: Buffer[]): TokenMetadataRegistered {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: TokenMetadataRegisteredFields = {
      tokenAddress: readPubkey(next("tokenAddress"), "tokenAddress"),
      decimals: readU8(next("decimals"), "decimals"),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new TokenMetadataRegistered(fields);
  }
}

type LinkTokenStartedFields = {
  tokenId: Buffer;
  destinationChain: string;
  sourceTokenAddress: PublicKey;
  destinationTokenAddress: Buffer;
  tokenManagerType: number;
  params: Buffer;
};
export class LinkTokenStarted extends BaseEvent {
  static override readonly EVENT_NAME = "LinkTokenStarted";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly tokenId: Buffer;
  readonly destinationChain: string;
  readonly sourceTokenAddress: PublicKey;
  readonly destinationTokenAddress: Buffer;
  readonly tokenManagerType: number;
  readonly params: Buffer;

  constructor(fields: LinkTokenStartedFields) {
    super();

    this.tokenId = fields.tokenId;
    this.destinationChain = fields.destinationChain;
    this.sourceTokenAddress = fields.sourceTokenAddress;
    this.destinationTokenAddress = fields.destinationTokenAddress;
    this.tokenManagerType = fields.tokenManagerType;
    this.params = fields.params;
  }
  static override deserialize(segments: Buffer[]): LinkTokenStarted {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: LinkTokenStartedFields = {
      tokenId: readFixedU8Array(next("tokenId"), 32, "tokenId"),
      destinationChain: readString(
        next("destinationChain"),
        "destinationChain"
      ),
      sourceTokenAddress: readPubkey(
        next("sourceTokenAddress"),
        "sourceTokenAddress"
      ),
      destinationTokenAddress: readVecU8(
        next("destinationTokenAddress"),
        "destinationTokenAddress"
      ),
      tokenManagerType: readU8(next("tokenManagerType"), "tokenManagerType"),
      params: readVecU8(next("params"), "params"),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new LinkTokenStarted(fields);
  }
}

type InterchainTokenDeploymentStartedFields = {
  tokenId: Buffer;
  tokenName: string;
  tokenSymbol: string;
  tokenDecimals: number;
  minter: Buffer;
  destinationChain: string;
};

export class InterchainTokenDeploymentStarted extends BaseEvent {
  static override readonly EVENT_NAME = "InterchainTokenDeploymentStarted";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly tokenId: Buffer;
  readonly tokenName: string;
  readonly tokenSymbol: string;
  readonly tokenDecimals: number;
  readonly minter: Buffer;
  readonly destinationChain: string;

  constructor(fields: InterchainTokenDeploymentStartedFields) {
    super();

    this.tokenId = fields.tokenId;
    this.tokenName = fields.tokenName;
    this.tokenSymbol = fields.tokenSymbol;
    this.tokenDecimals = fields.tokenDecimals;
    this.minter = fields.minter;
    this.destinationChain = fields.destinationChain;
  }
  static override deserialize(
    segments: Buffer[]
  ): InterchainTokenDeploymentStarted {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: InterchainTokenDeploymentStartedFields = {
      tokenId: readFixedU8Array(next("tokenId"), 32, "tokenId"),
      tokenName: readString(next("tokenName"), "tokenName"),
      tokenSymbol: readString(next("tokenSymbol"), "tokenSymbol"),
      tokenDecimals: readU8(next("tokenDecimals"), "tokenDecimals"),
      minter: readVecU8(next("minter"), "minter"),
      destinationChain: readString(
        next("destinationChain"),
        "destinationChain"
      ),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new InterchainTokenDeploymentStarted(fields);
  }
}

type TokenManagerDeployedFields = {
  tokenId: Buffer;
  tokenManager: PublicKey;
  tokenManagerType: number;
  params: Buffer;
};

export class TokenManagerDeployed extends BaseEvent {
  static override readonly EVENT_NAME = "TokenManagerDeployed";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly tokenId: Buffer;
  readonly tokenManager: PublicKey;
  readonly tokenManagerType: number;
  readonly params: Buffer;

  constructor(fields: TokenManagerDeployedFields) {
    super();

    this.tokenId = fields.tokenId;
    this.tokenManager = fields.tokenManager;
    this.tokenManagerType = fields.tokenManagerType;
    this.params = fields.params;
  }
  static override deserialize(segments: Buffer[]): TokenManagerDeployed {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: TokenManagerDeployedFields = {
      tokenId: readFixedU8Array(next("tokenId"), 32, "tokenId"),
      tokenManager: readPubkey(next("tokenManager"), "tokenManager"),
      tokenManagerType: readU8(next("tokenManagerType"), "tokenManagerType"),
      params: readVecU8(next("params"), "params"),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new TokenManagerDeployed(fields);
  }
}

type InterchainTokenDeployedFields = {
  tokenId: Buffer;
  tokenAddress: PublicKey;
  minter: PublicKey;
  name: string;
  symbol: string;
  decimals: number;
};

export class InterchainTokenDeployed extends BaseEvent {
  static override readonly EVENT_NAME = "InterchainTokenDeployed";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly tokenId: Buffer;
  readonly tokenAddress: PublicKey;
  readonly minter: PublicKey;
  readonly name: string;
  readonly symbol: string;
  readonly decimals: number;

  constructor(fields: InterchainTokenDeployedFields) {
    super();

    this.tokenId = fields.tokenId;
    this.tokenAddress = fields.tokenAddress;
    this.minter = fields.minter;
    this.name = fields.name;
    this.symbol = fields.symbol;
    this.decimals = fields.decimals;
  }
  static override deserialize(segments: Buffer[]): InterchainTokenDeployed {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: InterchainTokenDeployedFields = {
      tokenId: readFixedU8Array(next("tokenId"), 32, "tokenId"),
      tokenAddress: readPubkey(next("tokenAddress"), "tokenAddress"),
      minter: readPubkey(next("minter"), "minter"),
      name: readString(next("name"), "name"),
      symbol: readString(next("symbol"), "symbol"),
      decimals: readU8(next("decimals"), "decimals"),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new InterchainTokenDeployed(fields);
  }
}

type InterchainTokenIdClaimedFields = {
  tokenId: Buffer;
  deployer: PublicKey;
  salt: Buffer;
};

export class InterchainTokenIdClaimed extends BaseEvent {
  static override readonly EVENT_NAME = "InterchainTokenIdClaimed";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly tokenId: Buffer;
  readonly deployer: PublicKey;
  readonly salt: Buffer;

  constructor(fields: InterchainTokenIdClaimedFields) {
    super();

    this.tokenId = fields.tokenId;
    this.deployer = fields.deployer;
    this.salt = fields.salt;
  }

  static override deserialize(segments: Buffer[]): InterchainTokenIdClaimed {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: InterchainTokenIdClaimedFields = {
      tokenId: readFixedU8Array(next("tokenId"), 32, "tokenId"),
      deployer: readPubkey(next("deployer"), "deployer"),
      salt: readFixedU8Array(next("salt"), 32, "salt"),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new InterchainTokenIdClaimed(fields);
  }
}

type DeployRemoteInterchainTokenApprovalFields = {
  minter: PublicKey;
  deployer: PublicKey;
  tokenId: Buffer;
  destinationChain: string;
  destinationMinter: Buffer;
};

export class DeployRemoteInterchainTokenApproval extends BaseEvent {
  static override readonly EVENT_NAME = "DeployRemoteInterchainTokenApproval";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly minter: PublicKey;
  readonly deployer: PublicKey;
  readonly tokenId: Buffer;
  readonly destinationChain: string;
  readonly destinationMinter: Buffer;

  constructor(fields: DeployRemoteInterchainTokenApprovalFields) {
    super();

    this.minter = fields.minter;
    this.deployer = fields.deployer;
    this.tokenId = fields.tokenId;
    this.destinationChain = fields.destinationChain;
    this.destinationMinter = fields.destinationMinter;
  }

  static override deserialize(
    segments: Buffer[]
  ): DeployRemoteInterchainTokenApproval {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: DeployRemoteInterchainTokenApprovalFields = {
      minter: readPubkey(next("minter"), "minter"),
      deployer: readPubkey(next("deployer"), "deployer"),
      tokenId: readFixedU8Array(next("tokenId"), 32, "tokenId"),
      destinationChain: readString(
        next("destinationChain"),
        "destinationChain"
      ),
      destinationMinter: readVecU8(
        next("destinationMinter"),
        "destinationMinter"
      ),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new DeployRemoteInterchainTokenApproval(fields);
  }
}

type RevokeRemoteInterchainTokenApprovalFields = {
  minter: PublicKey;
  deployer: PublicKey;
  tokenId: Buffer;
  destinationChain: string;
};

export class RevokeRemoteInterchainTokenApproval extends BaseEvent {
  static override readonly EVENT_NAME = "RevokeRemoteInterchainTokenApproval";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly minter: PublicKey;
  readonly deployer: PublicKey;
  readonly tokenId: Buffer;
  readonly destinationChain: string;

  constructor(fields: RevokeRemoteInterchainTokenApprovalFields) {
    super();

    this.minter = fields.minter;
    this.deployer = fields.deployer;
    this.tokenId = fields.tokenId;
    this.destinationChain = fields.destinationChain;
  }

  static override deserialize(
    segments: Buffer[]
  ): RevokeRemoteInterchainTokenApproval {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: RevokeRemoteInterchainTokenApprovalFields = {
      minter: readPubkey(next("minter"), "minter"),
      deployer: readPubkey(next("deployer"), "deployer"),
      tokenId: readFixedU8Array(next("tokenId"), 32, "tokenId"),
      destinationChain: readString(
        next("destinationChain"),
        "destinationChain"
      ),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new RevokeRemoteInterchainTokenApproval(fields);
  }
}

type FlowLimitSetFields = {
  tokenId: Buffer;
  operator: PublicKey;
  flowLimit: number;
};

export class FlowLimitSet extends BaseEvent {
  static override readonly EVENT_NAME = "FlowLimitSet";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly tokenId: Buffer;
  readonly operator: PublicKey;
  readonly flowLimit: number;

  constructor(fields: FlowLimitSetFields) {
    super();

    this.tokenId = fields.tokenId;
    this.operator = fields.operator;
    this.flowLimit = fields.flowLimit;
  }

  static override deserialize(segments: Buffer[]): FlowLimitSet {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: FlowLimitSetFields = {
      tokenId: readFixedU8Array(next("tokenId"), 32, "tokenId"),
      operator: readPubkey(next("operator"), "operator"),
      flowLimit: readU64LE(next("flowLimit"), "flowLimit"),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new FlowLimitSet(fields);
  }
}

type TrustedChainSetFields = { chainName: string };

export class TrustedChainSet extends BaseEvent {
  static override readonly EVENT_NAME = "TrustedChainSet";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly chainName: string;

  constructor(fields: TrustedChainSetFields) {
    super();
    this.chainName = fields.chainName;
  }

  static override deserialize(segments: Buffer[]): TrustedChainSet {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: TrustedChainSetFields = {
      chainName: readString(next("chainName"), "chainName"),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new TrustedChainSet(fields);
  }
}

type TrustedChainRemovedFields = { chainName: string };

export class TrustedChainRemoved extends BaseEvent {
  static override readonly EVENT_NAME = "TrustedChainRemoved";
  static override readonly DISC = calculateDiscriminant(this.EVENT_NAME);
  readonly chainName: string;

  constructor(fields: TrustedChainRemovedFields) {
    super();

    this.chainName = fields.chainName;
  }
  static override deserialize(segments: Buffer[]): TrustedChainRemoved {
    let i = 0;
    const next = (f: string): Buffer => {
      if (i >= segments.length) throw new MissingDataError(f);
      return segments[i++];
    };
    const fields: TrustedChainRemovedFields = {
      chainName: readString(next("chainName"), "chainName"),
    };
    if (i < segments.length)
      throw new TrailingSegmentsError(segments.length - i);
    return new TrustedChainRemoved(fields);
  }
}

export const ITS_KNOWN_EVENT_CLASSES: EventClassType[] = [
  InterchainTransfer,
  InterchainTransferReceived,
  TokenMetadataRegistered,
  LinkTokenStarted,
  InterchainTokenDeploymentStarted,
  TokenManagerDeployed,
  InterchainTokenDeployed,
  InterchainTokenIdClaimed,
  DeployRemoteInterchainTokenApproval,
  RevokeRemoteInterchainTokenApproval,
  FlowLimitSet,
  TrustedChainSet,
  TrustedChainRemoved,
];

// Map DISC (hex string) to the corresponding event class constructor
export const ITS_EVENT_PARSER_MAP: EventParserMap = new Map(
  ITS_KNOWN_EVENT_CLASSES.map((cls) => [discToHex(cls.DISC), cls])
);
