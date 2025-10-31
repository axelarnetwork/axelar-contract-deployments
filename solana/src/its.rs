use anchor_lang::InstructionData;
use clap::{Args, Parser, Subcommand};
use eyre::eyre;
use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction as SolanaTransaction;

use crate::config::Config;
use crate::types::{SerializableSolanaTransaction, SolanaTransactionParams};
use crate::utils::{
    ADDRESS_KEY, CHAINS_KEY, CONFIG_ACCOUNT_KEY, CONTRACTS_KEY, ITS_KEY, OPERATOR_KEY,
    UPGRADE_AUTHORITY_KEY, fetch_latest_blockhash,
    read_json_file_from_path, write_json_to_file_path,
};

const ITS_SEED: &[u8] = b"interchain-token-service";
const TOKEN_MANAGER_SEED: &[u8] = b"token-manager";
const INTERCHAIN_TOKEN_SEED: &[u8] = b"interchain-token";
const PREFIX_INTERCHAIN_TOKEN_SALT: &[u8] = b"interchain-token-salt";
const PREFIX_CANONICAL_TOKEN_SALT: &[u8] = b"canonical-token-salt";
const PREFIX_CUSTOM_TOKEN_SALT: &[u8] = b"solana-custom-token-salt";

#[derive(Debug, Clone, Copy, borsh::BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
enum TokenManagerType {
    NativeInterchainToken = 0,
    MintBurnFrom = 1,
    LockUnlock = 2,
    LockUnlockFee = 3,
    MintBurn = 4,
    Gateway = 5,
}

#[derive(borsh::BorshDeserialize, Debug)]
struct TokenManager {
    token_address: Pubkey,
    ty: TokenManagerType,
    flow_slot: FlowSlot,
}

#[derive(borsh::BorshDeserialize, Debug)]
struct FlowSlot {
    flow_limit: Option<u64>,
}

fn find_its_root_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ITS_SEED], &solana_axelar_its::id())
}

fn find_token_manager_pda(its_root_pda: &Pubkey, token_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[TOKEN_MANAGER_SEED, its_root_pda.as_ref(), token_id],
        &solana_axelar_its::id(),
    )
}

fn find_interchain_token_pda(its_root_pda: &Pubkey, token_id: &[u8]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[INTERCHAIN_TOKEN_SEED, its_root_pda.as_ref(), token_id],
        &solana_axelar_its::id(),
    )
}

fn canonical_interchain_token_id(token_address: &Pubkey) -> [u8; 32] {
    solana_sdk::keccak::hashv(&[PREFIX_CANONICAL_TOKEN_SALT, token_address.as_ref()]).0
}

fn interchain_token_id(deployer: &Pubkey, salt: &[u8; 32]) -> [u8; 32] {
    let prefix_hash = solana_sdk::keccak::hashv(&[PREFIX_INTERCHAIN_TOKEN_SALT, salt]).0;
    solana_sdk::keccak::hashv(&[&prefix_hash, deployer.as_ref()]).0
}

fn linked_token_id(sender: &Pubkey, salt: &[u8; 32]) -> [u8; 32] {
    solana_sdk::keccak::hashv(&[PREFIX_CUSTOM_TOKEN_SALT, sender.as_ref(), salt]).0
}

fn not_implemented_error() -> eyre::Result<Vec<Instruction>> {
    eyre::bail!("This instruction is not yet implemented in the new Anchor ITS program. The Anchor program currently only supports: Initialize, SetPauseStatus, SetTrustedChain, and RemoveTrustedChain.")
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialize the Interchain Token Service (ITS) on Solana
    Init(InitArgs),

    /// Pause the Interchain Token Service on Solana, blocking incoming GMP calls and
    /// custom/canonical token registration
    Pause,

    /// Unpause the Interchain Token Service on Solana, allowing incoming GMP calls and
    /// custom/canonical token registration
    /// registration
    Unpause,

    /// Whitelists a chain on the Interchain Token Service
    SetTrustedChain(TrustedChainArgs),

    /// Removes a chain from the Interchain Token Service whitelist
    RemoveTrustedChain(TrustedChainArgs),

    /// Approve deploying a remote interchain token with a specific minter
    ApproveDeployRemoteInterchainToken(ApproveDeployRemoteInterchainTokenArgs),

    /// Revoke approval for deploying a remote interchain token
    RevokeDeployRemoteInterchainToken(RevokeDeployRemoteInterchainTokenArgs),

    /// Register a canonical token as an interchain token
    RegisterCanonicalInterchainToken(RegisterCanonicalInterchainTokenArgs),

    /// Deploy a canonical interchain token on a remote chain
    DeployRemoteCanonicalInterchainToken(DeployRemoteCanonicalInterchainTokenArgs),

    /// Deploy a new interchain token on Solana
    DeployInterchainToken(DeployInterchainTokenArgs),

    /// Deploy an existing interchain token to a remote chain
    DeployRemoteInterchainToken(DeployRemoteInterchainTokenArgs),

    /// Deploy an existing interchain token to a remote chain with a specific minter
    DeployRemoteInterchainTokenWithMinter(DeployRemoteInterchainTokenWithMinterArgs),

    /// Register token metadata with the Interchain Token Service Hub
    RegisterTokenMetadata(RegisterTokenMetadataArgs),

    /// Register a custom token with the Interchain Token Service
    RegisterCustomToken(RegisterCustomTokenArgs),

    /// Link a local token to a remote token
    LinkToken(LinkTokenArgs),

    /// Transfer interchain tokens
    InterchainTransfer(InterchainTransferArgs),

    /// Transfer interchain tokens to a contract and call it
    CallContractWithInterchainToken(CallContractWithInterchainTokenArgs),

    /// Transfer interchain tokens to a contract and call it using offchain data (recommended for
    /// payloads that exceed the Solana transaction size limit)
    CallContractWithInterchainTokenOffchainData(CallContractWithInterchainTokenOffchainDataArgs),

    /// Set the flow limit for an interchain token
    SetFlowLimit(SetFlowLimitArgs),

    /// Transfer the Interchain Token Service operatorship to another account
    TransferOperatorship(TransferOperatorshipArgs),

    /// Pose transfer of operatorship of the Interchain Token Service to another account
    ProposeOperatorship(TransferOperatorshipArgs), // Uses same args as transfer

    /// Accept an existing proposal for the transfer of operatorship of the Interchain Token
    /// Service from another account
    AcceptOperatorship(AcceptOperatorshipArgs),

    /// TokenManager specific commands
    #[clap(subcommand)]
    TokenManager(TokenManagerCommand),

    /// Interchain Token specific commands
    #[clap(subcommand)]
    InterchainToken(InterchainTokenCommand),
}

/// Commands for querying ITS related data
#[derive(Subcommand, Debug)]
pub(crate) enum QueryCommands {
    /// Get TokenManager details
    TokenManager(TokenManagerArgs),
}

#[derive(Args, Debug)]
pub(crate) struct TokenManagerArgs {
    /// The interchain token ID associated with the mint
    token_id: String,
}

#[derive(Subcommand, Debug)]
pub(crate) enum TokenManagerCommand {
    /// Set the flow limit for an Interchain Token on a TokenManager
    SetFlowLimit(TokenManagerSetFlowLimitArgs),

    /// Add the flow limiter role on a TokenManager to an account
    AddFlowLimiter(TokenManagerAddFlowLimiterArgs),

    /// Remove the flow limiter role on a TokenManager from an account
    RemoveFlowLimiter(TokenManagerRemoveFlowLimiterArgs),

    /// Transfer operatorship of a TokenManager to another account
    TransferOperatorship(TokenManagerTransferOperatorshipArgs),

    /// Porpose transfer of operatorship of a TokenManager to another account
    ProposeOperatorship(TokenManagerProposeOperatorshipArgs),

    /// Accept an existing proposal for the transfer of operatorship of a TokenManager from another account
    AcceptOperatorship(TokenManagerAcceptOperatorshipArgs),

    /// Handover mint authority of an SPL token to the TokenManager
    HandoverMintAuthority(TokenManagerHandoverMintAuthorityArgs),
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerSetFlowLimitArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The flow limit to set for the Interchain Token
    #[clap(long)]
    flow_limit: u64,

    /// The account with operator role on the TokenManager
    #[clap(long)]
    operator: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerAddFlowLimiterArgs {
    /// The account to add as a flow limiter
    #[clap(long)]
    adder: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to add as a flow limiter
    #[clap(long)]
    flow_limiter: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerRemoveFlowLimiterArgs {
    /// The account to remove as a flow limiter
    #[clap(long)]
    remover: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to remove as a flow limiter
    #[clap(long)]
    flow_limiter: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerTransferOperatorshipArgs {
    /// The account that sends the operatorship transfer
    #[clap(long)]
    sender: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to transfer operatorship to
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerProposeOperatorshipArgs {
    /// The account that proposes the operatorship transfer
    #[clap(long)]
    proposer: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to propose operatorship transfer to
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerAcceptOperatorshipArgs {
    /// The account that accepts the operatorship transfer
    #[clap(long)]
    accepter: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to accept operatorship transfer from
    #[clap(long)]
    from: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerHandoverMintAuthorityArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The authority account
    #[clap(long)]
    authority: Option<Pubkey>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum InterchainTokenCommand {
    /// Mint interchain tokens (requires minter role)
    Mint(InterchainTokenMintArgs),

    /// Transfer mintership of an interchain token (requires minter role)
    TransferMintership(InterchainTokenTransferMintershipArgs),

    /// Propose mintership transfer for an interchain token (requires minter role)
    ProposeMintership(InterchainTokenProposeMintershipArgs),

    /// Accept mintership transfer for an interchain token (requires minter role)
    AcceptMintership(InterchainTokenAcceptMintershipArgs),
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTokenMintArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The token account to which the tokens will be minted
    #[clap(long)]
    to: Pubkey,

    /// The amount of tokens to mint
    #[clap(long)]
    amount: String,
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTokenTransferMintershipArgs {
    /// The account that sends the minter role transfer
    #[clap(long)]
    sender: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to which the minter role will be transferred
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTokenProposeMintershipArgs {
    /// The account that proposes the minter role transfer
    #[clap(long)]
    proposer: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to which the minter role transfer will be proposed
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTokenAcceptMintershipArgs {
    /// The account that accepts the minter role transfer
    #[clap(long)]
    accepter: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account from which the minter role transfer proposal will be accepted
    #[clap(long)]
    from: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct InitArgs {
    /// The operator account for the Interchain Token Service
    #[clap(short, long)]
    operator: Pubkey,

    /// The chain name for the Interchain Token Service
    #[clap(long)]
    chain_name: String,

    /// The ITS hub address on the Axelar network
    #[clap(long)]
    its_hub_address: String,
}

#[derive(Parser, Debug)]
pub(crate) struct SetPauseStatusArgs {
    /// The pause status to set for the Interchain Token Service
    #[clap(short, long, required = true)]
    paused: bool,
}

#[derive(Parser, Debug)]
pub(crate) struct TrustedChainArgs {
    /// The name of the chain to set as trusted
    chain_name: String,

    /// The authority account (ITS operator or upgrade authority)
    #[clap(long)]
    authority: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct ApproveDeployRemoteInterchainTokenArgs {
    /// The account authorized to deploy the remote interchain token
    #[clap(long)]
    deployer: Pubkey,

    /// The salt for the approval
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],

    /// The chain which the remote interchain token will be deployed on
    #[clap(long)]
    destination_chain: String,

    /// The address to receive the minter role on the token deployed on destination chain
    #[clap(long)]
    destination_minter: String,

    /// The account with minter role on the token manager
    #[clap(long)]
    minter: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct RevokeDeployRemoteInterchainTokenArgs {
    /// The account that was initially authorized to deploy the remote interchain token
    #[clap(long)]
    deployer: Pubkey,

    /// The salt for the approval
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],

    /// The chain which the remote interchain token would be deployed on
    #[clap(long)]
    destination_chain: String,

    /// The account with minter role on the token manager
    #[clap(long)]
    minter: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct RegisterCanonicalInterchainTokenArgs {
    /// The mint account of the canonical token
    #[clap(long)]
    mint: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct DeployRemoteCanonicalInterchainTokenArgs {
    /// The mint account of the canonical token
    #[clap(long)]
    mint: Pubkey,

    /// The chain which the remote interchain token will be deployed on
    #[clap(long)]
    destination_chain: String,

    /// The amount of gas to pay for the cross-chain transaction
    #[clap(long)]
    gas_value: u64,

    /// Optional AxelarGasService program id on Solana
    #[clap(long)]
    gas_service: Option<Pubkey>,

    /// Optional AxelarGasService config account on Solana
    #[clap(long)]
    gas_config_account: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct DeployInterchainTokenArgs {
    /// The salt used to derive the interchain token id
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],

    /// The name of the interchain token
    #[clap(long)]
    name: String,

    /// The symbol of the interchain token
    #[clap(long)]
    symbol: String,

    /// The number of decimals for the interchain token
    #[clap(long)]
    decimals: u8,

    /// Initial supply of the interchain token
    #[clap(long)]
    initial_supply: String,

    /// Optional mint account for the interchain token. Required if initial_supply is zero
    #[clap(long)]
    minter: Option<Pubkey>,

    /// The account that will deploy the interchain token
    #[clap(long)]
    deployer: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct DeployRemoteInterchainTokenArgs {
    /// The salt used to derive the interchain token id
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],

    /// The chain which the remote interchain token will be deployed on
    #[clap(long)]
    destination_chain: String,

    /// The amount of gas to pay for the cross-chain transaction
    #[clap(long)]
    gas_value: u64,

    /// Optional AxelarGasService program id on Solana
    #[clap(long)]
    gas_service: Option<Pubkey>,

    /// Optional AxelarGasService config account on Solana
    #[clap(long)]
    gas_config_account: Option<Pubkey>,

    /// The account that will deploy the remote interchain token
    #[clap(long)]
    deployer: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct DeployRemoteInterchainTokenWithMinterArgs {
    /// The salt used to derive the interchain token id
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],

    /// The account that has the minter role on the interchain token on Solana
    #[clap(long)]
    minter: Pubkey,

    /// The chain which the remote interchain token will be deployed on
    #[clap(long)]
    destination_chain: String,

    /// The address to receive the minter role on the token deployed on destination chain
    #[clap(long)]
    destination_minter: String,

    /// The amount of gas to pay for the cross-chain transaction
    #[clap(long)]
    gas_value: u64,

    /// Optional AxelarGasService program id on Solana
    #[clap(long)]
    gas_service: Option<Pubkey>,

    /// Optional AxelarGasService config account on Solana
    #[clap(long)]
    gas_config_account: Option<Pubkey>,

    /// The account that will deploy the remote interchain token
    #[clap(long)]
    deployer: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct RegisterTokenMetadataArgs {
    /// The mint account being registered whose metadata should be registered
    #[clap(long)]
    mint: Pubkey,

    /// The amount of gas to pay for the cross-chain transaction
    #[clap(long)]
    gas_value: u64,

    /// Optional AxelarGasService program id on Solana
    #[clap(long)]
    gas_service: Option<Pubkey>,

    /// Optional AxelarGasService config account on Solana
    #[clap(long)]
    gas_config_account: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct RegisterCustomTokenArgs {
    /// The salt used to derive the interchain token id
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],

    /// The mint to register
    #[clap(long)]
    mint: Pubkey,

    /// The TokenManager type to use for this token
    #[clap(long, value_parser = parse_token_manager_type)]
    token_manager_type: TokenManagerType,

    /// An optional account to receive the operator role on the TokenManager associated with the token
    #[clap(long)]
    operator: Option<Pubkey>,

    /// The account that will register the custom token
    #[clap(long)]
    deployer: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct LinkTokenArgs {
    /// The salt used to derive the interchain token id
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],

    /// The chain on which the token should be linked
    #[clap(long)]
    destination_chain: String,

    /// The address of the token on the destination chain to link
    #[clap(long, value_parser = parse_hex_vec)]
    destination_token_address: Vec<u8>,

    /// The TokenManager type to use for this token
    #[clap(long, value_parser = parse_token_manager_type)]
    token_manager_type: TokenManagerType,

    /// Additional arguments for the link, depending on the chain specific implementation
    #[clap(long, value_parser = parse_hex_vec)]
    link_params: Vec<u8>,

    /// The amount of gas to pay for the cross-chain transaction
    #[clap(long)]
    gas_value: u64,

    /// Optional AxelarGasService program id on Solana
    #[clap(long)]
    gas_service: Option<Pubkey>,

    /// Optional AxelarGasService config account on Solana
    #[clap(long)]
    gas_config_account: Option<Pubkey>,

    /// The account that will link the token
    #[clap(long)]
    deployer: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTransferArgs {
    /// The token account from which tokens should transferred
    #[clap(long)]
    source_account: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The chain to which the tokens should be transferred
    #[clap(long)]
    destination_chain: String,

    /// The address on the destination chain to which the tokens should be transferred
    #[clap(long)]
    destination_address: String,

    /// The amount of tokens to transfer (supports fractional amounts like 123.55)
    #[clap(long)]
    amount: String,

    /// The amount of gas to pay for the cross-chain transaction
    #[clap(long)]
    gas_value: u64,

    /// Optional AxelarGasService program id on Solana
    #[clap(long)]
    gas_service: Option<Pubkey>,

    /// Optional AxelarGasService config account on Solana
    #[clap(long)]
    gas_config_account: Option<Pubkey>,

    /// Optional timestamp for the transaction. If not provided, the current time will be used.
    /// This is used to track the token flow. Attention must be paid when generating the
    /// transaction for offline signing, when this value should be set to the expected time the
    /// transaction will be broadcasted.
    #[clap(long)]
    timestamp: Option<i64>,

    /// The authority account (owner or delegate of the source account)
    #[clap(long)]
    authority: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct CallContractWithInterchainTokenArgs {
    /// The token account from which tokens should transferred
    #[clap(long)]
    source_account: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The chain to which the tokens should be transferred
    #[clap(long)]
    destination_chain: String,

    /// The address on the destination chain to which the tokens should be transferred and data
    /// sent
    #[clap(long)]
    destination_address: String,

    /// The amount of tokens to transfer
    #[clap(long)]
    amount: String,

    /// The call data to be sent to the contract on the destination chain
    #[clap(long, value_parser = parse_hex_vec)]
    data: Vec<u8>,

    /// The amount of gas to pay for the cross-chain transaction
    #[clap(long)]
    gas_value: u64,

    /// Optional AxelarGasService program id on Solana
    #[clap(long)]
    gas_service: Option<Pubkey>,

    /// Optional AxelarGasService config account on Solana
    #[clap(long)]
    gas_config_account: Option<Pubkey>,

    /// Optional timestamp for the transaction. If not provided, the current time will be used.
    /// This is used to track the token flow. Attention must be paid when generating the
    /// transaction for offline signing, when this value should be set to the expected time the
    /// transaction will be broadcasted.
    #[clap(long)]
    timestamp: Option<i64>,

    /// The authority account (owner or delegate of the source account)
    #[clap(long)]
    authority: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct CallContractWithInterchainTokenOffchainDataArgs {
    /// The token account from which tokens should transferred
    #[clap(long)]
    source_account: Pubkey,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The chain to which the tokens should be transferred
    #[clap(long)]
    destination_chain: String,

    /// The address on the destination chain to which the tokens should be transferred and data
    /// sent
    #[clap(long)]
    destination_address: String,

    /// The amount of tokens to transfer
    #[clap(long)]
    amount: String,

    /// The call data to be sent to the contract on the destination chain
    #[clap(long, value_parser = parse_hex_vec)]
    data: Vec<u8>,

    /// The amount of gas to pay for the cross-chain transaction
    #[clap(long)]
    gas_value: u64,

    /// Optional AxelarGasService program id on Solana
    #[clap(long)]
    gas_service: Option<Pubkey>,

    /// Optional AxelarGasService config account on Solana
    #[clap(long)]
    gas_config_account: Option<Pubkey>,

    /// Optional timestamp for the transaction. If not provided, the current time will be used.
    /// This is used to track the token flow. Attention must be paid when generating the
    /// transaction for offline signing, when this value should be set to the expected time the
    /// transaction will be broadcasted.
    #[clap(long)]
    timestamp: Option<i64>,

    /// The authority account (owner or delegate of the source account)
    #[clap(long)]
    authority: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct SetFlowLimitArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The flow limit to set for the Interchain Token
    #[clap(long)]
    flow_limit: u64,

    /// The operator account
    #[clap(long)]
    operator: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct TransferOperatorshipArgs {
    /// The account that sends the operatorship transfer
    #[clap(long)]
    sender: Pubkey,

    /// The account that proposes the operatorship transfer
    #[clap(long)]
    proposer: Pubkey,

    /// The account from which the operatorship will be transferred
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct AcceptOperatorshipArgs {
    /// The account to which the operatorship will be accepted
    #[clap(long)]
    role_receiver: Pubkey,

    /// The account from which the operatorship will be accepted
    #[clap(long)]
    from: Pubkey,
}

fn hash_salt(s: &str) -> eyre::Result<[u8; 32]> {
    Ok(solana_sdk::keccak::hash(s.as_bytes()).0)
}

fn parse_hex_vec(s: &str) -> Result<Vec<u8>, hex::FromHexError> {
    hex::decode(s.trim_start_matches("0x"))
}

fn parse_hex_bytes32(s: &str) -> eyre::Result<[u8; 32]> {
    let decoded: [u8; 32] = hex::decode(s.trim_start_matches("0x"))?
        .try_into()
        .map_err(|_| eyre!("Invalid hex string length. Expected 32 bytes."))?;

    Ok(decoded)
}

fn parse_token_manager_type(s: &str) -> Result<TokenManagerType, String> {
    match s.to_lowercase().as_str() {
        "lockunlock" | "lock_unlock" => Ok(TokenManagerType::LockUnlock),
        "mintburn" | "mint_burn" => Ok(TokenManagerType::MintBurn),
        "mintburnfrom" | "mint_burn_from" => Ok(TokenManagerType::MintBurnFrom),
        "lockunlockfee" | "lock_unlock_fee" => Ok(TokenManagerType::LockUnlockFee),
        "nativeinterchaintoken" | "native_interchain_token" => Ok(TokenManagerType::NativeInterchainToken),
        "gateway" => Ok(TokenManagerType::Gateway),
        _ => Err(format!("Invalid token manager type: {s}")),
    }
}

fn get_token_program_from_mint(mint: &Pubkey, config: &Config) -> eyre::Result<Pubkey> {
    let rpc_client = RpcClient::new(config.url.clone());
    let mint_account = rpc_client.get_account(mint)?;
    Ok(mint_account.owner)
}

fn get_token_decimals(mint: &Pubkey, config: &Config) -> eyre::Result<u8> {
    use solana_sdk::program_pack::Pack;
    use spl_token::state::Mint as TokenMint;
    use spl_token_2022::state::Mint as Token2022Mint;

    let rpc_client = RpcClient::new(config.url.clone());
    let mint_account = rpc_client.get_account(mint)?;

    match mint_account.owner.to_string().as_str() {
        crate::utils::TOKEN_2022_PROGRAM_ID => {
            let mint_data = Token2022Mint::unpack(&mint_account.data)
                .map_err(|_| eyre!("Failed to parse Token-2022 mint data"))?;
            Ok(mint_data.decimals)
        }
        crate::utils::SPL_TOKEN_PROGRAM_ID => {
            let mint_data = TokenMint::unpack(&mint_account.data)
                .map_err(|_| eyre!("Failed to parse SPL Token mint data"))?;
            Ok(mint_data.decimals)
        }
        _ => Err(eyre!("Unsupported token program: {}", mint_account.owner)),
    }
}

fn get_mint_from_token_manager(token_id: &[u8; 32], config: &Config) -> eyre::Result<Pubkey> {
    use borsh::BorshDeserialize as _;

    let rpc_client = RpcClient::new(config.url.clone());
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, token_id);
    let account = rpc_client.get_account(&token_manager_pda)?;
    let token_manager = TokenManager::try_from_slice(&account.data)?;
    Ok(token_manager.token_address)
}

pub(crate) fn build_instruction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config),
        Commands::Pause => set_pause_status(fee_payer, SetPauseStatusArgs { paused: true }),
        Commands::Unpause => set_pause_status(fee_payer, SetPauseStatusArgs { paused: false }),
        Commands::SetTrustedChain(set_trusted_chain_args) => {
            set_trusted_chain(fee_payer, set_trusted_chain_args, config)
        }
        Commands::RemoveTrustedChain(args) => remove_trusted_chain(fee_payer, args),
        Commands::ApproveDeployRemoteInterchainToken(args) => {
            approve_deploy_remote_interchain_token(fee_payer, args, config)
        }
        Commands::RevokeDeployRemoteInterchainToken(args) => {
            revoke_deploy_remote_interchain_token(fee_payer, args)
        }
        Commands::RegisterCanonicalInterchainToken(args) => {
            register_canonical_interchain_token(fee_payer, args, config)
        }
        Commands::DeployRemoteCanonicalInterchainToken(args) => {
            deploy_remote_canonical_interchain_token(fee_payer, args)
        }
        Commands::DeployInterchainToken(args) => deploy_interchain_token(fee_payer, args),
        Commands::DeployRemoteInterchainToken(args) => {
            deploy_remote_interchain_token(fee_payer, args)
        }
        Commands::DeployRemoteInterchainTokenWithMinter(args) => {
            deploy_remote_interchain_token_with_minter(fee_payer, args, config)
        }
        Commands::RegisterTokenMetadata(args) => register_token_metadata(fee_payer, args),
        Commands::RegisterCustomToken(args) => register_custom_token(fee_payer, args, config),
        Commands::LinkToken(args) => link_token(fee_payer, args),
        Commands::InterchainTransfer(args) => interchain_transfer(fee_payer, args, config),
        Commands::CallContractWithInterchainToken(args) => {
            call_contract_with_interchain_token(fee_payer, args, config)
        }
        Commands::CallContractWithInterchainTokenOffchainData(args) => {
            call_contract_with_interchain_token_offchain_data(fee_payer, args, config)
        }
        Commands::SetFlowLimit(args) => set_flow_limit(fee_payer, args),
        Commands::TransferOperatorship(args) => transfer_operatorship(fee_payer, args),
        Commands::ProposeOperatorship(args) => propose_operatorship(fee_payer, args),
        Commands::AcceptOperatorship(args) => accept_operatorship(fee_payer, args),
        Commands::TokenManager(command) => match command {
            TokenManagerCommand::SetFlowLimit(args) => {
                token_manager_set_flow_limit(fee_payer, args)
            }
            TokenManagerCommand::AddFlowLimiter(args) => {
                token_manager_add_flow_limiter(fee_payer, args)
            }
            TokenManagerCommand::RemoveFlowLimiter(args) => {
                token_manager_remove_flow_limiter(fee_payer, args)
            }
            TokenManagerCommand::TransferOperatorship(args) => {
                token_manager_transfer_operatorship(fee_payer, args)
            }
            TokenManagerCommand::ProposeOperatorship(args) => {
                token_manager_propose_operatorship(fee_payer, args)
            }
            TokenManagerCommand::AcceptOperatorship(args) => {
                token_manager_accept_operatorship(fee_payer, args)
            }
            TokenManagerCommand::HandoverMintAuthority(args) => {
                token_manager_handover_mint_authority(fee_payer, args, config)
            }
        },
        Commands::InterchainToken(command) => match command {
            InterchainTokenCommand::Mint(args) => interchain_token_mint(fee_payer, args, config),
            InterchainTokenCommand::TransferMintership(args) => {
                interchain_token_transfer_mintership(fee_payer, args)
            }
            InterchainTokenCommand::ProposeMintership(args) => {
                interchain_token_propose_mintership(fee_payer, args)
            }
            InterchainTokenCommand::AcceptMintership(args) => {
                interchain_token_accept_mintership(fee_payer, args)
            }
        },
    }
}

pub(crate) fn build_transaction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<SerializableSolanaTransaction>> {
    let instructions = build_instruction(fee_payer, command, config)?;

    // Get blockhash
    let blockhash = fetch_latest_blockhash(&config.url)?;

    // Create a transaction for each individual instruction
    let mut serializable_transactions = Vec::with_capacity(instructions.len());

    for instruction in instructions {
        // Build message and transaction with blockhash for a single instruction
        let message = solana_sdk::message::Message::new_with_blockhash(
            &[instruction],
            Some(fee_payer),
            &blockhash,
        );
        let transaction = SolanaTransaction::new_unsigned(message);

        // Create the transaction parameters
        // Note: Nonce account handling is done in generate_from_transactions
        // rather than here, so each transaction gets the nonce instruction prepended
        let params = SolanaTransactionParams {
            fee_payer: fee_payer.to_string(),
            recent_blockhash: Some(blockhash.to_string()),
            nonce_account: None,
            nonce_authority: None,
            blockhash_for_message: blockhash.to_string(),
        };

        // Create a serializable transaction
        let serializable_tx = SerializableSolanaTransaction::new(transaction, params);
        serializable_transactions.push(serializable_tx);
    }

    Ok(serializable_transactions)
}

fn init(
    fee_payer: &Pubkey,
    init_args: InitArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mut chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    let (its_root_pda, _) = find_its_root_pda();
    let program_data = solana_sdk::bpf_loader_upgradeable::get_program_data_address(&solana_axelar_its::id());

    let (user_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            its_root_pda.as_ref(),
            init_args.operator.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    chains_info[CHAINS_KEY][&config.chain][CONTRACTS_KEY][ITS_KEY] = serde_json::json!({
        ADDRESS_KEY: solana_axelar_its::id().to_string(),
        CONFIG_ACCOUNT_KEY: its_root_pda.to_string(),
        OPERATOR_KEY: init_args.operator.to_string(),
        UPGRADE_AUTHORITY_KEY: fee_payer.to_string(),
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;
    let ix_data = solana_axelar_its::instruction::Initialize {
        chain_name: init_args.chain_name,
        its_hub_address: init_args.its_hub_address,
    }.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts: vec![
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new_readonly(program_data, false),
            AccountMeta::new(its_root_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(init_args.operator, true),
            AccountMeta::new(user_roles_pda, false),
        ],
        data: ix_data,
    }])
}

fn set_pause_status(
    fee_payer: &Pubkey,
    set_pause_args: SetPauseStatusArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (its_root_pda, _) = find_its_root_pda();
    let (user_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            its_root_pda.as_ref(),
            fee_payer.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    let ix_data = solana_axelar_its::instruction::SetPauseStatus {
        paused: set_pause_args.paused,
    }.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts: vec![
            AccountMeta::new(its_root_pda, false),
            AccountMeta::new_readonly(*fee_payer, true),
            AccountMeta::new_readonly(user_roles_pda, false),
        ],
        data: ix_data,
    }])
}

fn set_trusted_chain(
    fee_payer: &Pubkey,
    set_trusted_chain_args: TrustedChainArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    if set_trusted_chain_args.chain_name.is_empty() {
        eyre::bail!("Chain name cannot be empty");
    }

    let authority = set_trusted_chain_args.authority.unwrap_or(*fee_payer);
    let mut instructions = Vec::new();
    if set_trusted_chain_args.chain_name == "all" {
        let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;

        if let serde_json::Value::Object(ref chains) = chains_info[CHAINS_KEY] {
            for chain in chains.keys() {
                println!("Creating instruction to set {chain} as trusted on Solana ITS");
                let (its_root_pda, _) = find_its_root_pda();
                let (user_roles_pda, _) = Pubkey::find_program_address(
                    &[
                        b"user-roles",
                        its_root_pda.as_ref(),
                        authority.as_ref(),
                    ],
                    &solana_axelar_its::id(),
                );

                let ix_data = solana_axelar_its::instruction::SetTrustedChain {
                    chain_name: chain.clone(),
                }.data();

                instructions.push(Instruction {
                    program_id: solana_axelar_its::id(),
                    accounts: vec![
                        AccountMeta::new(its_root_pda, false),
                        AccountMeta::new_readonly(authority, true),
                        AccountMeta::new_readonly(user_roles_pda, false),
                    ],
                    data: ix_data,
                });
            }
        } else {
            eyre::bail!("Failed to load all chains from chains info JSON file");
        }
    } else {
        let (its_root_pda, _) = find_its_root_pda();
        let (user_roles_pda, _) = Pubkey::find_program_address(
            &[
                b"user-roles",
                its_root_pda.as_ref(),
                authority.as_ref(),
            ],
            &solana_axelar_its::id(),
        );

        let ix_data = solana_axelar_its::instruction::SetTrustedChain {
            chain_name: set_trusted_chain_args.chain_name,
        }.data();

        instructions.push(Instruction {
            program_id: solana_axelar_its::id(),
            accounts: vec![
                AccountMeta::new(its_root_pda, false),
                AccountMeta::new_readonly(authority, true),
                AccountMeta::new_readonly(user_roles_pda, false),
            ],
            data: ix_data,
        });
    }

    Ok(instructions)
}

fn remove_trusted_chain(
    fee_payer: &Pubkey,
    remove_trusted_chain_args: TrustedChainArgs,
) -> eyre::Result<Vec<Instruction>> {
    let authority = remove_trusted_chain_args.authority.unwrap_or(*fee_payer);
    let (its_root_pda, _) = find_its_root_pda();
    let (user_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            its_root_pda.as_ref(),
            authority.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    let ix_data = solana_axelar_its::instruction::RemoveTrustedChain {
        chain_name: remove_trusted_chain_args.chain_name,
    }.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts: vec![
            AccountMeta::new(its_root_pda, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(user_roles_pda, false),
        ],
        data: ix_data,
    }])
}

fn approve_deploy_remote_interchain_token(
    _fee_payer: &Pubkey,
    _args: ApproveDeployRemoteInterchainTokenArgs,
    _config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn revoke_deploy_remote_interchain_token(
    _fee_payer: &Pubkey,
    _args: RevokeDeployRemoteInterchainTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn register_canonical_interchain_token(
    _fee_payer: &Pubkey,
    args: RegisterCanonicalInterchainTokenArgs,
    _config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let token_id = canonical_interchain_token_id(&args.mint);

    println!("------------------------------------------");
    println!("\u{1FA99} Token details:");
    println!();
    println!("- Interchain Token ID: {}", hex::encode(token_id));
    println!("- Mint Address: {}", args.mint);
    println!("------------------------------------------");

    not_implemented_error()
}

fn deploy_remote_canonical_interchain_token(
    _fee_payer: &Pubkey,
    _args: DeployRemoteCanonicalInterchainTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn deploy_interchain_token(
    fee_payer: &Pubkey,
    args: DeployInterchainTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    let _raw_supply =
        crate::utils::parse_decimal_string_to_raw_units(&args.initial_supply, args.decimals)?;

    let token_id = interchain_token_id(fee_payer, &args.salt);
    let (its_root_pda, _) = find_its_root_pda();
    let (mint, _) = find_interchain_token_pda(&its_root_pda, &token_id);

    println!("------------------------------------------");
    println!("\u{1FA99} Token details:");
    println!();
    println!("- Interchain Token ID: {}", hex::encode(token_id));
    println!("- Mint Address: {mint}");
    println!("- Human Amount: {} {}", args.initial_supply, args.symbol);
    println!("- Decimals: {}", args.decimals);
    println!("------------------------------------------");

    not_implemented_error()
}

fn deploy_remote_interchain_token(
    _fee_payer: &Pubkey,
    _args: DeployRemoteInterchainTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn deploy_remote_interchain_token_with_minter(
    _fee_payer: &Pubkey,
    _args: DeployRemoteInterchainTokenWithMinterArgs,
    _config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn register_token_metadata(
    _fee_payer: &Pubkey,
    _args: RegisterTokenMetadataArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn register_custom_token(
    fee_payer: &Pubkey,
    args: RegisterCustomTokenArgs,
    _config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let token_id = linked_token_id(fee_payer, &args.salt);

    println!("------------------------------------------");
    println!("\u{1FA99} Token details:");
    println!();
    println!("- Interchain Token ID: {}", hex::encode(token_id));
    println!("- Mint Address: {}", args.mint);
    println!("------------------------------------------");

    not_implemented_error()
}

fn link_token(_fee_payer: &Pubkey, _args: LinkTokenArgs) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn interchain_transfer(
    _fee_payer: &Pubkey,
    args: InterchainTransferArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mint = get_mint_from_token_manager(&args.token_id, config)?;
    let decimals = get_token_decimals(&mint, config)?;

    let _raw_amount = crate::utils::parse_decimal_string_to_raw_units(&args.amount, decimals)?;

    println!("------------------------------------------");
    println!("\u{1FA99} Transfer details:");
    println!();
    println!("- Human Amount: {} tokens", args.amount);
    println!("- Decimals: {decimals}");
    println!("- Destination Chain: {}", args.destination_chain);
    println!("- Destination Address: {}", args.destination_address);
    println!("------------------------------------------");

    not_implemented_error()
}

fn call_contract_with_interchain_token(
    _fee_payer: &Pubkey,
    args: CallContractWithInterchainTokenArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mint = get_mint_from_token_manager(&args.token_id, config)?;
    let decimals = get_token_decimals(&mint, config)?;

    let _raw_amount = crate::utils::parse_decimal_string_to_raw_units(&args.amount, decimals)?;

    println!("------------------------------------------");
    println!("\u{1FA99} Contract call details:");
    println!();
    println!("- Human Amount: {} tokens", args.amount);
    println!("- Decimals: {decimals}");
    println!("- Destination Chain: {}", args.destination_chain);
    println!("- Destination Address: {}", args.destination_address);
    println!("------------------------------------------");

    not_implemented_error()
}

fn call_contract_with_interchain_token_offchain_data(
    _fee_payer: &Pubkey,
    args: CallContractWithInterchainTokenOffchainDataArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mint = get_mint_from_token_manager(&args.token_id, config)?;
    let decimals = get_token_decimals(&mint, config)?;

    let _raw_amount = crate::utils::parse_decimal_string_to_raw_units(&args.amount, decimals)?;

    println!("------------------------------------------");
    println!("\u{1FA99} Offchain contract call details:");
    println!();
    println!("- Human Amount: {} tokens", args.amount);
    println!("- Decimals: {decimals}");
    println!("- Destination Chain: {}", args.destination_chain);
    println!("- Destination Address: {}", args.destination_address);
    println!("------------------------------------------");

    not_implemented_error()
}

fn set_flow_limit(_fee_payer: &Pubkey, _args: SetFlowLimitArgs) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn transfer_operatorship(
    _fee_payer: &Pubkey,
    _args: TransferOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn propose_operatorship(
    _fee_payer: &Pubkey,
    _args: TransferOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn accept_operatorship(
    _fee_payer: &Pubkey,
    _args: AcceptOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn token_manager_set_flow_limit(
    _fee_payer: &Pubkey,
    _args: TokenManagerSetFlowLimitArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn token_manager_add_flow_limiter(
    _fee_payer: &Pubkey,
    _args: TokenManagerAddFlowLimiterArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn token_manager_remove_flow_limiter(
    _fee_payer: &Pubkey,
    _args: TokenManagerRemoveFlowLimiterArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn token_manager_transfer_operatorship(
    _fee_payer: &Pubkey,
    _args: TokenManagerTransferOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn token_manager_propose_operatorship(
    _fee_payer: &Pubkey,
    _args: TokenManagerProposeOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn token_manager_accept_operatorship(
    _fee_payer: &Pubkey,
    _args: TokenManagerAcceptOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn token_manager_handover_mint_authority(
    _fee_payer: &Pubkey,
    _args: TokenManagerHandoverMintAuthorityArgs,
    _config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn interchain_token_mint(
    _fee_payer: &Pubkey,
    _args: InterchainTokenMintArgs,
    _config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn interchain_token_transfer_mintership(
    _fee_payer: &Pubkey,
    _args: InterchainTokenTransferMintershipArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn interchain_token_propose_mintership(
    _fee_payer: &Pubkey,
    _args: InterchainTokenProposeMintershipArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

fn interchain_token_accept_mintership(
    _fee_payer: &Pubkey,
    _args: InterchainTokenAcceptMintershipArgs,
) -> eyre::Result<Vec<Instruction>> {
    not_implemented_error()
}

pub(crate) fn query(command: QueryCommands, config: &Config) -> eyre::Result<()> {
    match command {
        QueryCommands::TokenManager(mint_args) => get_token_manager(mint_args, config),
    }
}

fn get_token_manager(args: TokenManagerArgs, config: &Config) -> eyre::Result<()> {
    use borsh::BorshDeserialize as _;

    let rpc_client = RpcClient::new(config.url.clone());
    let (its_root_pda, _) = find_its_root_pda();
    let token_id: [u8; 32] = hex::decode(args.token_id.trim_start_matches("0x"))?
        .try_into()
        .map_err(|vec| eyre!("invalid token id: {vec:?}"))?;
    let (token_manager_pda, _) =
        find_token_manager_pda(&its_root_pda, &token_id);
    let account = rpc_client.get_account(&token_manager_pda)?;
    let token_manager = TokenManager::try_from_slice(&account.data)?;

    println!("------------------------------------------");
    println!("\u{1FA99} TokenManager details:");
    println!();
    println!("- Interchain Token ID: {}", args.token_id);
    println!("- Mint Address: {}", token_manager.token_address);
    println!("- Type: {:#?}", token_manager.ty);
    println!("- Flow Limit: {:?}", token_manager.flow_slot.flow_limit);
    println!("------------------------------------------");

    Ok(())
}
