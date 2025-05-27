import { axelarSolanaItsProgram } from "../../generated/axelar-solana-its/src";
import { AnchorProvider, BN, Program } from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  SystemProgram,
} from "@solana/web3.js";
import {
  canonicalInterchainTokenId,
  findCallContractSigningPda,
  findDeploymentApprovalPda,
  findFlowSlotPda,
  findInterchainTokenPda,
  findItsRootPda,
  findMetadataPda,
  findTokenManagerPda,
  findProposalPda,
  findUserRolesPda,
  interchainTokenId,
  linkedTokenId,
  TOKEN_METADATA_PROGRAM_ID,
} from "./pda.js";
import { utils } from "ethers";
import { AXELAR_SOLANA_GATEWAY_PROGRAM_ID } from "../../generated/axelar-solana-gateway/src";

const BPF_UPGRADE_LOADER_ID = new PublicKey(
  "BPFLoaderUpgradeab1e11111111111111111111111",
);
const MESSAGE_TYPE_INTERCHAIN_TRANSFER = 0;
const MESSAGE_TYPE_SEND_TO_HUB = 3;

export enum TokenManagerType {
  NativeInterchainToken,
  MintBurnFrom,
  LockUnlock,
  LockUnlockFee,
  MintBurn,
}

function tokenManagerTypeToInternalRepr(type: TokenManagerType) {
  switch (type) {
    case TokenManagerType.NativeInterchainToken:
      return { nativeInterchainToken: {} };
    case TokenManagerType.MintBurnFrom:
      return { mintBurnFrom: {} };
    case TokenManagerType.LockUnlock:
      return { lockUnlock: {} };
    case TokenManagerType.LockUnlockFee:
      return { lockUnlockFee: {} };
    case TokenManagerType.MintBurn:
      return { mintBurn: {} };
  }
}

function flowEpochWithTimestamp(timestamp: number): number {
  const EPOCH_TIME = 6 * 60 * 60;
  return Math.floor(timestamp / EPOCH_TIME);
}

export class ItsInstructions {
  readonly program: ReturnType<typeof axelarSolanaItsProgram>;
  readonly gatewayRootPda: PublicKey;
  readonly itsRootPda: PublicKey;
  readonly programDataAddress: PublicKey;
  readonly interchainToken: InterchainTokenInstructions;
  readonly tokenManager: TokenManagerInstructions;

  constructor(
    itsId: PublicKey,
    gatewayRootPda: PublicKey,
    provider: AnchorProvider,
  ) {
    this.program = axelarSolanaItsProgram({ programId: itsId, provider });
    this.gatewayRootPda = gatewayRootPda;
    this.itsRootPda = findItsRootPda(gatewayRootPda)[0];
    this.programDataAddress = PublicKey.findProgramAddressSync(
      [this.program.programId.toBytes()],
      BPF_UPGRADE_LOADER_ID,
    )[0];
    this.interchainToken = new InterchainTokenInstructions(
      itsId,
      gatewayRootPda,
      provider,
    );
    this.tokenManager = new TokenManagerInstructions(
      itsId,
      gatewayRootPda,
      provider,
    );
  }

  initialize(params: {
    payer: PublicKey;
    operator: PublicKey;
    chainName: string;
    itsHubAddress: string;
  }): ReturnType<Program["methods"]["initialize"]> {
    const [userRolesPda] = findUserRolesPda(this.itsRootPda, params.payer);
    return this.program.methods
      .initialize(params.chainName, params.itsHubAddress)
      .accounts({
        payer: params.payer,
        programDataAddress: this.programDataAddress,
        itsRootPda: this.itsRootPda,
        systemProgram: SystemProgram.programId,
        operator: params.operator,
        userRolesPda,
      });
  }

  setPaused(params: {
    payer: PublicKey;
    paused: boolean;
  }): ReturnType<Program["methods"]["setPaused"]> {
    return this.program.methods.setPauseStatus(params.paused).accounts({
      payer: params.payer,
      programDataAddress: this.programDataAddress,
      itsRootPda: this.itsRootPda,
      systemProgram: SystemProgram.programId,
    });
  }

  setTrustedChain(params: {
    payer: PublicKey;
    chainName: string;
  }): ReturnType<Program["methods"]["setTrustedChain"]> {
    return this.program.methods.setTrustedChain(params.chainName).accounts({
      payer: params.payer,
      programDataAddress: this.programDataAddress,
      itsRootPda: this.itsRootPda,
      systemProgram: SystemProgram.programId,
    });
  }

  removeTrustedChain(params: {
    payer: PublicKey;
    chainName: string;
  }): ReturnType<Program["methods"]["removeTrustedChain"]> {
    return this.program.methods.removeTrustedChain(params.chainName).accounts({
      payer: params.payer,
      programDataAddress: this.programDataAddress,
      itsRootPda: this.itsRootPda,
      systemProgram: SystemProgram.programId,
    });
  }

  approveDeployRemoteInterchainToken(params: {
    payer: PublicKey;
    deployer: PublicKey;
    salt: number[];
    destinationChain: string;
    destinationMinter: Uint8Array;
  }): ReturnType<Program["methods"]["approveDeployRemoteInterchainToken"]> {
    const tokenId = interchainTokenId(
      params.deployer,
      Buffer.from(params.salt),
    );
    const [tokenManagerPda] = findTokenManagerPda(this.itsRootPda, tokenId);
    const [rolesPda] = findUserRolesPda(tokenManagerPda, params.payer);
    const [deployApprovalPda] = findDeploymentApprovalPda(
      params.payer,
      tokenId,
      params.destinationChain,
    );
    return this.program.methods
      .approveDeployRemoteInterchainToken(
        params.deployer,
        params.salt,
        params.destinationChain,
        Buffer.from(params.destinationMinter),
      )
      .accounts({
        payer: params.payer,
        tokenManagerPda,
        rolesPda,
        deployApprovalPda,
        systemProgram: SystemProgram.programId,
      });
  }

  revokeDeployRemoteInterchainToken(params: {
    payer: PublicKey;
    deployer: PublicKey;
    salt: number[];
    destinationChain: string;
  }): ReturnType<Program["methods"]["approveDeployRemoteInterchainToken"]> {
    const tokenId = interchainTokenId(
      params.deployer,
      Buffer.from(params.salt),
    );
    const [deployApprovalPda] = findDeploymentApprovalPda(
      params.payer,
      tokenId,
      params.destinationChain,
    );
    return this.program.methods
      .revokeDeployRemoteInterchainToken(
        params.deployer,
        params.salt,
        params.destinationChain,
      )
      .accounts({
        payer: params.payer,
        deployApprovalPda,
        systemProgram: SystemProgram.programId,
      });
  }

  registerCanonicalInterchainToken(params: {
    payer: PublicKey;
    mint: PublicKey;
    tokenProgram: PublicKey;
  }): ReturnType<Program["methods"]["registerCanonicalInterchainToken"]> {
    const tokenId = canonicalInterchainTokenId(params.mint);
    const [tokenManagerPda] = findTokenManagerPda(this.itsRootPda, tokenId);
    const tokenManagerAta = getAssociatedTokenAddressSync(
      params.mint,
      tokenManagerPda,
      true,
      params.tokenProgram,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    const [tokenMetadataAccount] = findMetadataPda(params.mint);
    const [userRolesPda] = findUserRolesPda(tokenManagerPda, this.itsRootPda);

    return this.program.methods.registerCanonicalInterchainToken().accounts({
      payer: params.payer,
      tokenMetadataAccount,
      systemProgram: SystemProgram.programId,
      itsRootPda: this.itsRootPda,
      tokenManagerPda,
      mint: params.mint,
      tokenManagerAta,
      tokenProgram: params.tokenProgram,
      splAssociatedTokenAccount: ASSOCIATED_TOKEN_PROGRAM_ID,
      itsUserRolesPda: userRolesPda,
      rent: SYSVAR_RENT_PUBKEY,
    });
  }

  deployRemoteCanonicalInterchainToken(params: {
    payer: PublicKey;
    mint: PublicKey;
    destinationChain: string;
    gasValue: BN;
    gasService: PublicKey;
    gasConfigPda: PublicKey;
    tokenProgram: PublicKey;
  }): ReturnType<Program["methods"]["deployRemoteCanonicalInterchainToken"]> {
    const [callContractSigningPda, signingPdaBump] =
      findCallContractSigningPda();
    const [tokenMetadataAccount] = findMetadataPda(params.mint);

    return this.program.methods
      .deployRemoteCanonicalInterchainToken(
        params.destinationChain,
        params.gasValue,
        signingPdaBump,
      )
      .accounts({
        payer: params.payer,
        mint: params.mint,
        metadataAccount: tokenMetadataAccount,
        axelarSolanaGateway: AXELAR_SOLANA_GATEWAY_PROGRAM_ID,
        gasConfigPda: params.gasConfigPda,
        gasService: params.gasService,
        systemProgram: SystemProgram.programId,
        itsRootPda: this.itsRootPda,
        callContractSigningPda,
        id: this.program.programId,
      });
  }

  deployInterchainToken(params: {
    payer: PublicKey;
    salt: number[];
    name: string;
    symbol: string;
    decimals: number;
    initialSupply: BN;
    minter?: PublicKey;
  }): ReturnType<Program["methods"]["deployInterchainToken"]> {
    const tokenId = interchainTokenId(params.payer, Buffer.from(params.salt));
    const [tokenManagerPda] = findTokenManagerPda(this.itsRootPda, tokenId);
    const [interchainTokenPda] = findInterchainTokenPda(
      this.itsRootPda,
      tokenId,
    );
    const tokenManagerAta = getAssociatedTokenAddressSync(
      interchainTokenPda,
      tokenManagerPda,
      true,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    const payerAta = getAssociatedTokenAddressSync(
      interchainTokenPda,
      params.payer,
      true,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    );

    const [itsUserRolesPda] = findUserRolesPda(
      tokenManagerPda,
      this.itsRootPda,
    );
    const [tokenMetadataAccount] = findMetadataPda(interchainTokenPda);
    let minterRolesPda;

    if (params.minter) {
      [minterRolesPda] = findUserRolesPda(tokenManagerPda, params.minter);
    }

    return this.program.methods
      .deployInterchainToken(
        params.salt,
        params.name,
        params.symbol,
        params.decimals,
        params.initialSupply,
      )
      .accounts({
        payer: params.payer,
        systemProgram: SystemProgram.programId,
        itsRootPda: this.itsRootPda,
        tokenManagerPda,
        mint: interchainTokenPda,
        tokenManagerAta,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        splAssociatedTokenAccount: ASSOCIATED_TOKEN_PROGRAM_ID,
        itsUserRolesPda,
        rent: SYSVAR_RENT_PUBKEY,
        sysvarInstructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        mplTokenMetadata: TOKEN_METADATA_PROGRAM_ID,
        metadataAccount: tokenMetadataAccount,
        payerAta,
        minter: params.minter,
        minterRolesPda,
      });
  }

  deployRemoteInterchainToken(params: {
    payer: PublicKey;
    salt: number[];
    destinationChain: string;
    gasValue: BN;
    gasService: PublicKey;
    gasConfigPda: PublicKey;
  }): ReturnType<Program["methods"]["deployRemoteInterchainToken"]> {
    const tokenId = interchainTokenId(params.payer, Buffer.from(params.salt));
    const [interchainTokenPda] = findInterchainTokenPda(
      this.itsRootPda,
      tokenId,
    );
    const [callContractSigningPda, signingPdaBump] =
      findCallContractSigningPda();
    const [tokenMetadataAccount] = findMetadataPda(interchainTokenPda);

    return this.program.methods
      .deployRemoteInterchainToken(
        params.salt,
        params.destinationChain,
        params.gasValue,
        signingPdaBump,
      )
      .accounts({
        payer: params.payer,
        mint: interchainTokenPda,
        metadataAccount: tokenMetadataAccount,
        axelarSolanaGateway: AXELAR_SOLANA_GATEWAY_PROGRAM_ID,
        gasConfigPda: params.gasConfigPda,
        gasService: params.gasService,
        systemProgram: SystemProgram.programId,
        itsRootPda: this.itsRootPda,
        callContractSigningPda,
        id: this.program.programId,
      });
  }

  deployRemoteInterchainTokenWithMinter(params: {
    payer: PublicKey;
    salt: number[];
    minter: PublicKey;
    destinationChain: string;
    destinationMinter: Uint8Array;
    gasValue: BN;
    gasService: PublicKey;
    gasConfigPda: PublicKey;
  }): ReturnType<Program["methods"]["deployRemoteInterchainTokenWithMinter"]> {
    const tokenId = interchainTokenId(params.payer, Buffer.from(params.salt));
    const [interchainTokenPda] = findInterchainTokenPda(
      this.itsRootPda,
      tokenId,
    );
    const [callContractSigningPda, signingPdaBump] =
      findCallContractSigningPda();
    const [tokenMetadataAccount] = findMetadataPda(interchainTokenPda);
    const [deployApprovalPda] = findDeploymentApprovalPda(
      params.minter,
      tokenId,
      params.destinationChain,
    );
    const [tokenManagerPda] = findTokenManagerPda(this.itsRootPda, tokenId);
    const [minterRolesPda] = findUserRolesPda(tokenManagerPda, params.minter);

    return this.program.methods
      .deployRemoteInterchainTokenWithMinter(
        params.salt,
        params.destinationChain,
        Buffer.from(params.destinationMinter),
        params.gasValue,
        signingPdaBump,
      )
      .accounts({
        payer: params.payer,
        mint: interchainTokenPda,
        metadataAccount: tokenMetadataAccount,
        minter: params.minter,
        deployApproval: deployApprovalPda,
        minterRolesPda,
        tokenManagerPda,
        axelarSolanaGateway: AXELAR_SOLANA_GATEWAY_PROGRAM_ID,
        gasConfigPda: params.gasConfigPda,
        gasService: params.gasService,
        systemProgram: SystemProgram.programId,
        itsRootPda: this.itsRootPda,
        callContractSigningPda,
        id: this.program.programId,
      });
  }

  registerTokenMetadata(params: {
    payer: PublicKey;
    mint: PublicKey;
    tokenProgram: PublicKey;
    gasValue: BN;
    gasService: PublicKey;
    gasConfigPda: PublicKey;
  }): ReturnType<Program["methods"]["registerTokenMetadata"]> {
    const [callContractSigningPda, signingPdaBump] =
      findCallContractSigningPda();

    return this.program.methods
      .registerTokenMetadata(params.gasValue, signingPdaBump)
      .accounts({
        payer: params.payer,
        mint: params.mint,
        tokenProgram: params.tokenProgram,
        axelarSolanaGateway: AXELAR_SOLANA_GATEWAY_PROGRAM_ID,
        gasConfigPda: params.gasConfigPda,
        gasService: params.gasService,
        systemProgram: SystemProgram.programId,
        itsRootPda: this.itsRootPda,
        callContractSigningPda,
        id: this.program.programId,
      });
  }

  registerCustomToken(params: {
    payer: PublicKey;
    salt: number[];
    mint: PublicKey;
    tokenManagerType: TokenManagerType;
    tokenProgram: PublicKey;
    operator: PublicKey | null;
  }): ReturnType<Program["methods"]["registerCustomToken"]> {
    const tokenId = linkedTokenId(params.payer, Buffer.from(params.salt));
    const [tokenManagerPda] = findTokenManagerPda(this.itsRootPda, tokenId);
    const tokenManagerAta = getAssociatedTokenAddressSync(
      params.mint,
      tokenManagerPda,
      true,
      params.tokenProgram,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    const [tokenMetadataAccount] = findMetadataPda(params.mint);
    const [itsUserRolesPda] = findUserRolesPda(
      tokenManagerPda,
      this.itsRootPda,
    );
    const operatorRolesPda = params.operator
      ? findUserRolesPda(tokenManagerPda, params.operator)[0]
      : null;

    return this.program.methods
      .registerCustomToken(
        params.salt,
        tokenManagerTypeToInternalRepr(params.tokenManagerType),
        params.operator,
      )
      .accounts({
        payer: params.payer,
        tokenMetadataAccount,
        systemProgram: SystemProgram.programId,
        itsRootPda: this.itsRootPda,
        tokenManagerPda,
        mint: params.mint,
        tokenManagerAta,
        tokenProgram: params.tokenProgram,
        splAssociatedTokenAccount: ASSOCIATED_TOKEN_PROGRAM_ID,
        itsUserRolesPda,
        rent: SYSVAR_RENT_PUBKEY,
        operator: params.operator,
        operatorRolesPda,
      });
  }

  linkToken(params: {
    payer: PublicKey;
    salt: number[];
    destinationChain: string;
    destinationTokenAddress: Uint8Array;
    tokenManagerType: TokenManagerType;
    linkParams: Uint8Array;
    gasValue: BN;
    gasService: PublicKey;
    gasConfigPda: PublicKey;
  }): ReturnType<Program["methods"]["linkToken"]> {
    const [callContractSigningPda, signingPdaBump] =
      findCallContractSigningPda();
    const tokenId = linkedTokenId(params.payer, Buffer.from(params.salt));
    const [tokenManagerPda] = findTokenManagerPda(this.itsRootPda, tokenId);

    return this.program.methods
      .linkToken(
        params.salt,
        params.destinationChain,
        Buffer.from(params.destinationTokenAddress),
        tokenManagerTypeToInternalRepr(params.tokenManagerType),
        Buffer.from(params.linkParams),
        params.gasValue,
        signingPdaBump,
      )
      .accounts({
        payer: params.payer,
        tokenManagerPda,
        axelarSolanaGateway: AXELAR_SOLANA_GATEWAY_PROGRAM_ID,
        gasConfigPda: params.gasConfigPda,
        gasService: params.gasService,
        systemProgram: SystemProgram.programId,
        itsRootPda: this.itsRootPda,
        callContractSigningPda,
        id: this.program.programId,
      });
  }

  interchainTransfer(params: {
    payer: PublicKey;
    sourceAccount: PublicKey;
    authority: PublicKey | null;
    tokenId: number[];
    destinationChain: string;
    destinationAddress: Uint8Array;
    amount: BN;
    mint: PublicKey;
    tokenProgram: PublicKey;
    gasValue: BN;
    gasService: PublicKey;
    gasConfigPda: PublicKey;
  }): ReturnType<Program["methods"]["interchainTransfer"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );

    const timestamp = Math.floor(Date.now() / 1000);
    const flowEpoch = flowEpochWithTimestamp(timestamp);
    const [flowSlotPda] = findFlowSlotPda(tokenManagerPda, flowEpoch);

    const authority = params.authority ? params.authority! : tokenManagerPda;
    const tokenManagerAta = getAssociatedTokenAddressSync(
      params.mint,
      tokenManagerPda,
      true,
      params.tokenProgram,
    );
    const [callContractSigningPda, signingPdaBump] =
      findCallContractSigningPda();

    return this.program.methods
      .interchainTransfer(
        params.tokenId,
        params.destinationChain,
        Buffer.from(params.destinationAddress),
        params.amount,
        params.gasValue,
        signingPdaBump,
      )
      .accounts({
        payer: params.payer,
        authority,
        sourceAccount: params.sourceAccount,
        mint: params.mint,
        tokenManagerPda,
        tokenManagerAta,
        tokenProgram: params.tokenProgram,
        flowSlotPda,
        axelarSolanaGateway: AXELAR_SOLANA_GATEWAY_PROGRAM_ID,
        gasConfigPda: params.gasConfigPda,
        gasService: params.gasService,
        systemProgram: SystemProgram.programId,
        itsRootPda: this.itsRootPda,
        callContractSigningPda,
        id: this.program.programId,
      });
  }

  callContractWithInterchainToken(params: {
    payer: PublicKey;
    sourceAccount: PublicKey;
    authority: PublicKey | null;
    tokenId: number[];
    destinationChain: string;
    destinationAddress: Uint8Array;
    amount: BN;
    mint: PublicKey;
    data: Uint8Array;
    tokenProgram: PublicKey;
    gasValue: BN;
    gasService: PublicKey;
    gasConfigPda: PublicKey;
  }): ReturnType<Program["methods"]["callContractWithInterchainToken"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );

    const timestamp = Math.floor(Date.now() / 1000);
    const flowEpoch = flowEpochWithTimestamp(timestamp);
    const [flowSlotPda] = findFlowSlotPda(tokenManagerPda, flowEpoch);

    const authority = params.authority ? params.authority : tokenManagerPda;
    const tokenManagerAta = getAssociatedTokenAddressSync(
      params.mint,
      tokenManagerPda,
      true,
      params.tokenProgram,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    const [callContractSigningPda, signingPdaBump] =
      findCallContractSigningPda();

    return this.program.methods
      .callContractWithInterchainToken(
        params.tokenId,
        params.destinationChain,
        Buffer.from(params.destinationAddress),
        params.amount,
        Buffer.from(params.data),
        params.gasValue,
        signingPdaBump,
      )
      .accounts({
        payer: params.payer,
        authority,
        sourceAccount: params.sourceAccount,
        mint: params.mint,
        tokenManagerPda,
        tokenManagerAta,
        tokenProgram: params.tokenProgram,
        flowSlotPda,
        axelarSolanaGateway: AXELAR_SOLANA_GATEWAY_PROGRAM_ID,
        gasConfigPda: params.gasConfigPda,
        gasService: params.gasService,
        systemProgram: SystemProgram.programId,
        itsRootPda: this.itsRootPda,
        callContractSigningPda,
        id: this.program.programId,
      });
  }

  callContractWithInterchainTokenOffchainData(params: {
    payer: PublicKey;
    sourceAccount: PublicKey;
    authority: PublicKey | null;
    tokenId: number[];
    destinationChain: string;
    destinationAddress: Uint8Array;
    amount: BN;
    mint: PublicKey;
    data: Uint8Array;
    tokenProgram: PublicKey;
    gasValue: BN;
    gasService: PublicKey;
    gasConfigPda: PublicKey;
  }): [
    ReturnType<Program["methods"]["callContractWithInterchainToken"]>,
    Uint8Array,
  ] {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );

    const timestamp = Math.floor(Date.now() / 1000);
    const flowEpoch = flowEpochWithTimestamp(timestamp);
    const [flowSlotPda] = findFlowSlotPda(tokenManagerPda, flowEpoch);

    const authority = params.authority ? params.authority : tokenManagerPda;
    const tokenManagerAta = getAssociatedTokenAddressSync(
      params.mint,
      tokenManagerPda,
      true,
      params.tokenProgram,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    const [callContractSigningPda, signingPdaBump] =
      findCallContractSigningPda();

    const transferPayload = utils.defaultAbiCoder.encode(
      ["uint256", "bytes32", "bytes", "bytes", "uint256", "bytes"],
      [
        MESSAGE_TYPE_INTERCHAIN_TRANSFER,
        params.tokenId,
        utils.arrayify(params.sourceAccount.toBytes()),
        params.destinationAddress,
        params.amount,
        params.data,
      ],
    );

    const hubPayload = utils.defaultAbiCoder.encode(
      ["uint256", "string", "bytes"],
      [MESSAGE_TYPE_SEND_TO_HUB, params.destinationChain, transferPayload],
    );

    const payloadHash = utils.arrayify(utils.keccak256(hubPayload));

    return [
      this.program.methods
        .callContractWithInterchainTokenOffchainData(
          params.tokenId,
          params.destinationChain,
          Buffer.from(params.destinationAddress),
          params.amount,
          Array.from(payloadHash),
          params.gasValue,
          signingPdaBump,
        )
        .accounts({
          payer: params.payer,
          authority,
          sourceAccount: params.sourceAccount,
          mint: params.mint,
          tokenManagerPda,
          tokenManagerAta,
          tokenProgram: params.tokenProgram,
          flowSlotPda,
          axelarSolanaGateway: AXELAR_SOLANA_GATEWAY_PROGRAM_ID,
          gasConfigPda: params.gasConfigPda,
          gasService: params.gasService,
          systemProgram: SystemProgram.programId,
          itsRootPda: this.itsRootPda,
          callContractSigningPda,
          id: this.program.programId,
        }),
      utils.arrayify(hubPayload),
    ];
  }

  setFlowLimit(params: {
    payer: PublicKey;
    tokenId: number[];
    flowLimit: BN;
  }): ReturnType<Program["methods"]["setFlowLimit"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [itsUserRolesPda] = findUserRolesPda(this.itsRootPda, params.payer);
    const [tokenManagerUserRolesPda] = findUserRolesPda(
      tokenManagerPda,
      this.itsRootPda,
    );

    return this.program.methods.setFlowLimit(params.flowLimit).accounts({
      payer: params.payer,
      itsRootPda: this.itsRootPda,
      tokenManagerPda,
      itsUserRolesPda,
      tokenManagerUserRolesPda,
      systemProgram: SystemProgram.programId,
    });
  }

  transferOperatorship(params: {
    payer: PublicKey;
    newOperator: PublicKey;
  }): ReturnType<Program["methods"]["operatorTransferOperatorship"]> {
    const [destinationUserRolesPda, destinationRolesPdaBump] = findUserRolesPda(
      this.itsRootPda,
      params.newOperator,
    );
    const [payerRolesPda] = findUserRolesPda(this.itsRootPda, params.payer);

    return this.program.methods
      .operatorTransferOperatorship({
        roles: { operator: {} },
        destinationRolesPdaBump,
        proposalPdaBump: null,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: this.itsRootPda,
        destinationUserAccount: params.newOperator,
        destinationRolesAccount: destinationUserRolesPda,
        originUserAccount: params.payer,
        originRolesAccount: payerRolesPda,
      });
  }

  proposeOperatorship(params: {
    payer: PublicKey;
    newOperator: PublicKey;
  }): ReturnType<Program["methods"]["operatorProposeOperatorship"]> {
    const [destinationUserRolesPda, destinationRolesPdaBump] = findUserRolesPda(
      this.itsRootPda,
      params.newOperator,
    );
    const [payerRolesPda] = findUserRolesPda(this.itsRootPda, params.payer);
    const [proposalPda, proposalPdaBump] = findProposalPda(
      this.itsRootPda,
      params.payer,
      params.newOperator,
    );

    return this.program.methods
      .operatorProposeOperatorship({
        roles: { operator: {} },
        destinationRolesPdaBump,
        proposalPdaBump,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: this.itsRootPda,
        destinationUserAccount: params.newOperator,
        destinationRolesAccount: destinationUserRolesPda,
        originUserAccount: params.payer,
        originRolesAccount: payerRolesPda,
        proposalAccount: proposalPda,
      });
  }

  acceptOperatorship(params: {
    payer: PublicKey;
    oldOperator: PublicKey;
  }): ReturnType<Program["methods"]["operatorAcceptOperatorship"]> {
    const [payerRolesPda, payerRolesPdaBump] = findUserRolesPda(
      this.itsRootPda,
      params.payer,
    );
    const [oldOperatorRolesPda] = findUserRolesPda(
      this.itsRootPda,
      params.oldOperator,
    );
    const [proposalPda, proposalPdaBump] = findProposalPda(
      this.itsRootPda,
      params.oldOperator,
      params.payer,
    );

    return this.program.methods
      .operatorAcceptOperatorship({
        roles: { operator: {} },
        destinationRolesPdaBump: payerRolesPdaBump,
        proposalPdaBump,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: this.itsRootPda,
        destinationUserAccount: params.payer,
        destinationRolesAccount: payerRolesPda,
        originUserAccount: params.oldOperator,
        originRolesAccount: oldOperatorRolesPda,
        proposalAccount: proposalPda,
      });
  }
}

export class InterchainTokenInstructions {
  readonly program: ReturnType<typeof axelarSolanaItsProgram>;
  readonly gatewayRootPda: PublicKey;
  readonly itsRootPda: PublicKey;
  readonly programDataAddress: PublicKey;

  constructor(
    itsId: PublicKey,
    gatewayRootPda: PublicKey,
    provider: AnchorProvider,
  ) {
    this.program = axelarSolanaItsProgram({ programId: itsId, provider });
    this.gatewayRootPda = gatewayRootPda;
    this.itsRootPda = findItsRootPda(gatewayRootPda)[0];
    this.programDataAddress = PublicKey.findProgramAddressSync(
      [this.program.programId.toBytes()],
      BPF_UPGRADE_LOADER_ID,
    )[0];
  }

  mint(params: {
    tokenId: number[];
    mint: PublicKey;
    to: PublicKey;
    minter: PublicKey;
    tokenProgram: PublicKey;
    amount: BN;
  }): ReturnType<Program["methods"]["mint"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [minterRolesPda] = findUserRolesPda(tokenManagerPda, params.minter);

    return this.program.methods.interchainTokenMint(params.amount).accounts({
      mint: params.mint,
      destinationAccount: params.to,
      itsRootPda: this.itsRootPda,
      tokenManagerPda,
      minter: params.minter,
      minterRolesPda,
      tokenProgram: params.tokenProgram,
    });
  }

  transferMintership(params: {
    payer: PublicKey;
    tokenId: number[];
    newMinter: PublicKey;
  }): ReturnType<Program["methods"]["interchainTokenTransferMintership"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [destinationUserRolesPda, destinationRolesPdaBump] = findUserRolesPda(
      tokenManagerPda,
      params.newMinter,
    );
    const [payerRolesPda] = findUserRolesPda(tokenManagerPda, params.payer);

    return this.program.methods
      .interchainTokenTransferMintership({
        roles: { minter: {} },
        destinationRolesPdaBump,
        proposalPdaBump: null,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: this.itsRootPda,
        destinationUserAccount: params.newMinter,
        destinationRolesAccount: destinationUserRolesPda,
        originUserAccount: params.payer,
        originRolesAccount: payerRolesPda,
      });
  }

  proposeMintership(params: {
    payer: PublicKey;
    tokenId: number[];
    newMinter: PublicKey;
  }): ReturnType<Program["methods"]["interchainTokenProposeMintership"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [destinationUserRolesPda, destinationRolesPdaBump] = findUserRolesPda(
      tokenManagerPda,
      params.newMinter,
    );
    const [payerRolesPda] = findUserRolesPda(tokenManagerPda, params.payer);
    const [proposalPda, proposalPdaBump] = findProposalPda(
      tokenManagerPda,
      params.payer,
      params.newMinter,
    );

    return this.program.methods
      .interchainTokenProposeMintership({
        roles: { minter: {} },
        destinationRolesPdaBump,
        proposalPdaBump,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: this.itsRootPda,
        destinationUserAccount: params.newMinter,
        destinationRolesAccount: destinationUserRolesPda,
        originUserAccount: params.payer,
        originRolesAccount: payerRolesPda,
        proposalAccount: proposalPda,
      });
  }

  acceptMintership(params: {
    payer: PublicKey;
    tokenId: number[];
    oldMinter: PublicKey;
  }): ReturnType<Program["methods"]["interchainTokenAcceptMintership"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [payerRolesPda, payerRolesPdaBump] = findUserRolesPda(
      tokenManagerPda,
      params.payer,
    );
    const [oldMinterRolesPda] = findUserRolesPda(
      tokenManagerPda,
      params.oldMinter,
    );
    const [proposalPda, proposalPdaBump] = findProposalPda(
      tokenManagerPda,
      params.oldMinter,
      params.payer,
    );

    return this.program.methods
      .interchainTokenAcceptMintership({
        roles: { minter: {} },
        destinationRolesPdaBump: payerRolesPdaBump,
        proposalPdaBump,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: this.itsRootPda,
        destinationUserAccount: params.payer,
        destinationRolesAccount: payerRolesPda,
        originUserAccount: params.oldMinter,
        originRolesAccount: oldMinterRolesPda,
        proposalAccount: proposalPda,
      });
  }
}

export class TokenManagerInstructions {
  readonly program: ReturnType<typeof axelarSolanaItsProgram>;
  readonly gatewayRootPda: PublicKey;
  readonly itsRootPda: PublicKey;
  readonly programDataAddress: PublicKey;

  constructor(
    itsId: PublicKey,
    gatewayRootPda: PublicKey,
    provider: AnchorProvider,
  ) {
    this.program = axelarSolanaItsProgram({ programId: itsId, provider });
    this.gatewayRootPda = gatewayRootPda;
    this.itsRootPda = findItsRootPda(gatewayRootPda)[0];
    this.programDataAddress = PublicKey.findProgramAddressSync(
      [this.program.programId.toBytes()],
      BPF_UPGRADE_LOADER_ID,
    )[0];
  }

  handOverMintAuthority(params: {
    payer: PublicKey;
    tokenId: number[];
    mint: PublicKey;
    tokenProgram: PublicKey;
  }): ReturnType<Program["methods"]["tokenManagerHandOverMintAuthority"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [minterRolesPda] = findUserRolesPda(tokenManagerPda, params.payer);

    return this.program.methods
      .tokenManagerHandOverMintAuthority(params.tokenId)
      .accounts({
        payer: params.payer,
        mint: params.mint,
        itsRootPda: this.itsRootPda,
        tokenManagerPda,
        minterRolesPda,
        tokenProgram: params.tokenProgram,
        systemProgram: SystemProgram.programId,
      });
  }

  setFlowLimit(params: {
    payer: PublicKey;
    tokenId: number[];
    flowLimit: BN;
  }): ReturnType<Program["methods"]["tokenManagerSetFlowLimit"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [tokenManagerUserRolesPda] = findUserRolesPda(
      tokenManagerPda,
      params.payer,
    );
    const [itsUserRolesPda] = findUserRolesPda(this.itsRootPda, params.payer);

    return this.program.methods
      .tokenManagerSetFlowLimit(params.flowLimit)
      .accounts({
        payer: params.payer,
        itsRootPda: this.itsRootPda,
        tokenManagerPda,
        tokenManagerUserRolesPda,
        itsUserRolesPda,
        systemProgram: SystemProgram.programId,
      });
  }

  addFlowLimiter(params: {
    payer: PublicKey;
    tokenId: number[];
    flowLimiter: PublicKey;
  }): ReturnType<Program["methods"]["tokenManagerAddFlowLimiter"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [destinationUserRolesPda, destinationRolesPdaBump] = findUserRolesPda(
      tokenManagerPda,
      params.flowLimiter,
    );
    const [payerRolesPda] = findUserRolesPda(tokenManagerPda, params.payer);

    return this.program.methods
      .tokenManagerAddFlowLimiter({
        roles: { flowLimiter: {} },
        destinationRolesPdaBump,
        proposalPdaBump: null,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: tokenManagerPda,
        destinationUserAccount: params.flowLimiter,
        destinationRolesAccount: destinationUserRolesPda,
      });
  }

  removeFlowLimiter(params: {
    payer: PublicKey;
    tokenId: number[];
    flowLimiter: PublicKey;
  }): ReturnType<Program["methods"]["tokenManagerRemoveFlowLimiter"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [destinationUserRolesPda, destinationRolesPdaBump] = findUserRolesPda(
      tokenManagerPda,
      params.flowLimiter,
    );
    const [payerRolesPda] = findUserRolesPda(tokenManagerPda, params.payer);
    return this.program.methods
      .tokenManagerRemoveFlowLimiter({
        roles: { flowLimiter: {} },
        destinationRolesPdaBump,
        proposalPdaBump: null,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: tokenManagerPda,
        destinationUserAccount: params.flowLimiter,
        destinationRolesAccount: destinationUserRolesPda,
      });
  }

  transferOperatorship(params: {
    payer: PublicKey;
    tokenId: number[];
    newOperator: PublicKey;
  }): ReturnType<Program["methods"]["tokenManagerTransferOperatorship"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [destinationUserRolesPda, destinationRolesPdaBump] = findUserRolesPda(
      tokenManagerPda,
      params.newOperator,
    );
    const [payerRolesPda] = findUserRolesPda(tokenManagerPda, params.payer);

    return this.program.methods
      .tokenManagerTransferOperatorship({
        roles: { operator: {} },
        destinationRolesPdaBump,
        proposalPdaBump: null,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: this.itsRootPda,
        destinationUserAccount: params.newOperator,
        destinationRolesAccount: destinationUserRolesPda,
        originUserAccount: params.payer,
        originRolesAccount: payerRolesPda,
      });
  }

  proposeOperatorship(params: {
    payer: PublicKey;
    tokenId: number[];
    newOperator: PublicKey;
  }): ReturnType<Program["methods"]["tokenManagerProposeOperatorship"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [destinationUserRolesPda, destinationRolesPdaBump] = findUserRolesPda(
      tokenManagerPda,
      params.newOperator,
    );
    const [payerRolesPda] = findUserRolesPda(tokenManagerPda, params.payer);
    const [proposalPda, proposalPdaBump] = findProposalPda(
      tokenManagerPda,
      params.payer,
      params.newOperator,
    );

    return this.program.methods
      .tokenManagerProposeOperatorship({
        roles: { operator: {} },
        destinationRolesPdaBump,
        proposalPdaBump,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: this.itsRootPda,
        destinationUserAccount: params.newOperator,
        destinationRolesAccount: destinationUserRolesPda,
        originUserAccount: params.payer,
        originRolesAccount: payerRolesPda,
        proposalAccount: proposalPda,
      });
  }

  acceptOperatorship(params: {
    payer: PublicKey;
    tokenId: number[];
    oldOperator: PublicKey;
  }): ReturnType<Program["methods"]["tokenManagerAcceptOperatorship"]> {
    const [tokenManagerPda] = findTokenManagerPda(
      this.itsRootPda,
      Buffer.from(params.tokenId),
    );
    const [payerRolesPda, payerRolesPdaBump] = findUserRolesPda(
      tokenManagerPda,
      params.payer,
    );
    const [oldOperatorRolesPda] = findUserRolesPda(
      tokenManagerPda,
      params.oldOperator,
    );
    const [proposalPda, proposalPdaBump] = findProposalPda(
      tokenManagerPda,
      params.oldOperator,
      params.payer,
    );

    return this.program.methods
      .tokenManagerAcceptOperatorship({
        roles: { operator: {} },
        destinationRolesPdaBump: payerRolesPdaBump,
        proposalPdaBump,
      })
      .accounts({
        systemProgram: SystemProgram.programId,
        payer: params.payer,
        payerRolesAccount: payerRolesPda,
        resource: this.itsRootPda,
        destinationUserAccount: params.payer,
        destinationRolesAccount: payerRolesPda,
        originUserAccount: params.oldOperator,
        originRolesAccount: oldOperatorRolesPda,
        proposalAccount: proposalPda,
      });
  }
}
