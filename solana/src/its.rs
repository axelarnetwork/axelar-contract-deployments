use std::fs::File;
use std::io::Write;
use std::time::SystemTime;

use axelar_solana_its::state;
use clap::{Parser, Subcommand};
use eyre::eyre;
use serde::Deserialize;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction as SolanaTransaction;

use crate::config::Config;
use crate::types::{ChainNameOnAxelar, SerializableSolanaTransaction, SolanaTransactionParams};
use crate::utils::{
    decode_its_destination, fetch_latest_blockhash, read_json_file_from_path,
    write_json_to_file_path, ADDRESS_KEY, AXELAR_KEY, CHAINS_KEY, CONFIG_ACCOUNT_KEY,
    CONTRACTS_KEY, GAS_SERVICE_KEY, ITS_KEY, OPERATOR_KEY, UPGRADE_AUTHORITY_KEY,
};

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
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerAddFlowLimiterArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to add as a flow limiter
    #[clap(long)]
    flow_limiter: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerRemoveFlowLimiterArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to remove as a flow limiter
    #[clap(long)]
    flow_limiter: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerTransferOperatorshipArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to transfer operatorship to
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerProposeOperatorshipArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to propose operatorship transfer to
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct TokenManagerAcceptOperatorshipArgs {
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

    /// The mint whose authority will be handed over to the TokenManager
    #[clap(long)]
    mint: Pubkey,

    /// The token program which owns the mint (spl_token or spl_token_2022).
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,
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

    /// The mint account associated with the Interchain Token
    #[clap(long)]
    mint: Pubkey,

    /// The token account to which the tokens will be minted
    #[clap(long)]
    to: Pubkey,

    /// The token program which owns the the mint (spl_token or spl_token_2022).
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,

    /// The amount of tokens to mint
    #[clap(long)]
    amount: u64,
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTokenTransferMintershipArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to which the minter role will be transferred
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTokenProposeMintershipArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The account to which the minter role transfer will be proposed
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTokenAcceptMintershipArgs {
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
    #[clap(short, long)]
    chain_name: String,
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
}

#[derive(Parser, Debug)]
pub(crate) struct RegisterCanonicalInterchainTokenArgs {
    /// The mint account of the canonical token
    #[clap(long)]
    mint: Pubkey,

    /// The token program which owns the mint (spl_token or spl_token_2022).
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,
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
    initial_supply: u64,

    /// Optional mint account for the interchain token. Required if initial_supply is zero
    #[clap(long)]
    minter: Option<Pubkey>,
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
}

#[derive(Parser, Debug)]
pub(crate) struct RegisterTokenMetadataArgs {
    /// The mint account being registered whose metadata should be registered
    #[clap(long)]
    mint: Pubkey,

    /// The token program which owns the mint (spl_token or spl_token_2022).
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,

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
    token_manager_type: state::token_manager::Type,

    /// The token program which owns the mint (spl_token or spl_token_2022).
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,

    /// An optional account to receive the operator role on the TokenManager associated with the token
    #[clap(long)]
    operator: Option<Pubkey>,
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
    token_manager_type: state::token_manager::Type,

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
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTransferArgs {
    /// The token account from which tokens should transferred
    #[clap(long)]
    source_account: Pubkey,

    /// The authority with rights to transfer the tokens (i.e.: owner, delegate authority). If not
    /// set, tries to use the TokenManager PDA.
    #[clap(long)]
    authority: Option<Pubkey>,

    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The chain to which the tokens should be transferred
    #[clap(long)]
    destination_chain: String,

    /// The address on the destination chain to which the tokens should be transferred
    #[clap(long)]
    destination_address: String,

    /// The amount of tokens to transfer
    #[clap(long)]
    amount: u64,

    /// The mint account associated with the Interchain Token
    #[clap(long)]
    mint: Pubkey,

    /// The token program which owns the mint (spl_token or spl_token_2022).
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,

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
}

#[derive(Parser, Debug)]
pub(crate) struct CallContractWithInterchainTokenArgs {
    /// The token account from which tokens should transferred
    #[clap(long)]
    source_account: Pubkey,

    /// The authority with rights to transfer the tokens (i.e.: owner, delegate authority). If not
    /// set, tries to use the TokenManager PDA.
    #[clap(long)]
    authority: Option<Pubkey>,

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
    amount: u64,

    /// The mint account associated with the Interchain Token
    #[clap(long)]
    mint: Pubkey,

    /// The call data to be sent to the contract on the destination chain
    #[clap(long, value_parser = parse_hex_vec)]
    data: Vec<u8>,

    /// The token program to use for the mint. This can be either spl_token or spl_token_2022.
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,

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
}

#[derive(Parser, Debug)]
pub(crate) struct CallContractWithInterchainTokenOffchainDataArgs {
    /// The token account from which tokens should transferred
    #[clap(long)]
    source_account: Pubkey,

    /// The authority with rights to transfer the tokens (i.e.: owner, delegate authority). If not
    /// set, tries to use the TokenManager PDA.
    #[clap(long)]
    authority: Option<Pubkey>,

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
    amount: u64,

    /// The mint account associated with the Interchain Token
    #[clap(long)]
    mint: Pubkey,

    /// The call data to be sent to the contract on the destination chain
    #[clap(long, value_parser = parse_hex_vec)]
    data: Vec<u8>,

    /// The token program to use for the mint. This can be either spl_token or spl_token_2022.
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,

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
}

#[derive(Parser, Debug)]
pub(crate) struct SetFlowLimitArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The flow limit to set for the Interchain Token
    #[clap(long)]
    flow_limit: u64,
}

#[derive(Parser, Debug)]
pub(crate) struct TransferOperatorshipArgs {
    /// The account to which the operatorship will be transferred
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct AcceptOperatorshipArgs {
    /// The account from which the operatorship will be accepted
    #[clap(long)]
    from: Pubkey,
}

fn hash_salt(s: &str) -> eyre::Result<[u8; 32]> {
    Ok(solana_sdk::keccak::hash(s.as_bytes()).0)
}

fn parse_hex_vec(s: &str) -> Result<Vec<u8>, hex::FromHexError> {
    hex::decode(s.strip_prefix("0x").unwrap_or(s))
}

fn parse_hex_bytes32(s: &str) -> eyre::Result<[u8; 32]> {
    let decoded: [u8; 32] = hex::decode(s.strip_prefix("0x").unwrap_or(s))?
        .try_into()
        .map_err(|_| eyre!("Invalid hex string length. Expected 32 bytes."))?;

    Ok(decoded)
}

fn parse_token_program(s: &str) -> Result<Pubkey, String> {
    match s.to_lowercase().as_str() {
        "spl_token" => Ok(spl_token::id()),
        "spl_token_2022" => Ok(spl_token_2022::id()),
        _ => Err(format!("Invalid token program: {s}")),
    }
}

fn parse_token_manager_type(s: &str) -> Result<state::token_manager::Type, String> {
    match s.to_lowercase().as_str() {
        "lockunlock" | "lock_unlock" => Ok(state::token_manager::Type::LockUnlock),
        "mintburn" | "mint_burn" => Ok(state::token_manager::Type::MintBurn),
        "mintburnfrom" | "mint_burn_from" => Ok(state::token_manager::Type::MintBurnFrom),
        "lockunlockfee" | "lock_unlock_fee" => Ok(state::token_manager::Type::LockUnlockFee),
        _ => Err(format!("Invalid token manager type: {s}")),
    }
}

fn try_infer_gas_service_id(maybe_arg: Option<Pubkey>, config: &Config) -> eyre::Result<Pubkey> {
    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    if let Some(id) = maybe_arg {
        Ok(id)
    } else {
        let id = Pubkey::deserialize(
            &chains_info[ChainNameOnAxelar::from(config.network_type).0][CONTRACTS_KEY]
                [GAS_SERVICE_KEY][ADDRESS_KEY],
        ).map_err(|_| eyre!(
            "Could not get the gas service id from the chains info JSON file. Is it already deployed? \
            Please update the file or pass a value to --gas-service"))?;

        Ok(id)
    }
}

fn try_infer_gas_service_config_account(
    maybe_arg: Option<Pubkey>,
    config: &Config,
) -> eyre::Result<Pubkey> {
    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    if let Some(id) = maybe_arg {
        Ok(id)
    } else {
        let id = Pubkey::deserialize(
            &chains_info[ChainNameOnAxelar::from(config.network_type).0][CONTRACTS_KEY]
                [GAS_SERVICE_KEY][CONFIG_ACCOUNT_KEY],
        ).map_err(|_| eyre!(
            "Could not get the gas service config PDA from the chains info JSON file. Is it already deployed? \
            Please update the file or pass a value to --gas-config-account"))?;

        Ok(id)
    }
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
            register_canonical_interchain_token(fee_payer, args)
        }
        Commands::DeployRemoteCanonicalInterchainToken(args) => {
            deploy_remote_canonical_interchain_token(fee_payer, args, config)
        }
        Commands::DeployInterchainToken(args) => deploy_interchain_token(fee_payer, args),
        Commands::DeployRemoteInterchainToken(args) => {
            deploy_remote_interchain_token(fee_payer, args, config)
        }
        Commands::DeployRemoteInterchainTokenWithMinter(args) => {
            deploy_remote_interchain_token_with_minter(fee_payer, args, config)
        }
        Commands::RegisterTokenMetadata(args) => register_token_metadata(fee_payer, args, config),
        Commands::RegisterCustomToken(args) => register_custom_token(fee_payer, args),
        Commands::LinkToken(args) => link_token(fee_payer, args, config),
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
                token_manager_handover_mint_authority(fee_payer, args)
            }
        },
        Commands::InterchainToken(command) => match command {
            InterchainTokenCommand::Mint(args) => interchain_token_mint(fee_payer, args),
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
    let its_hub_address =
        String::deserialize(&chains_info[AXELAR_KEY][CONTRACTS_KEY][ITS_KEY][ADDRESS_KEY])?;
    let its_root_config = axelar_solana_its::find_its_root_pda().0;

    chains_info[CHAINS_KEY][ChainNameOnAxelar::from(config.network_type).0][CONTRACTS_KEY]
        [ITS_KEY] = serde_json::json!({
        ADDRESS_KEY: axelar_solana_gateway::id().to_string(),
        CONFIG_ACCOUNT_KEY: its_root_config.to_string(),
        UPGRADE_AUTHORITY_KEY: fee_payer.to_string(),
        OPERATOR_KEY: init_args.operator.to_string(),
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    Ok(vec![axelar_solana_its::instruction::initialize(
        *fee_payer,
        init_args.operator,
        ChainNameOnAxelar::from(config.network_type).0,
        its_hub_address,
    )?])
}

fn set_pause_status(
    fee_payer: &Pubkey,
    set_pause_args: SetPauseStatusArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![axelar_solana_its::instruction::set_pause_status(
        *fee_payer,
        set_pause_args.paused,
    )?])
}

fn set_trusted_chain(
    fee_payer: &Pubkey,
    set_trusted_chain_args: TrustedChainArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    if set_trusted_chain_args.chain_name.is_empty() {
        eyre::bail!("Chain name cannot be empty");
    }

    let mut instructions = Vec::new();
    if set_trusted_chain_args.chain_name == "all" {
        let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;

        if let serde_json::Value::Object(ref chains) = chains_info[CHAINS_KEY] {
            for chain in chains.keys() {
                println!("Creating instruction to set {chain} as trusted on Solana ITS");
                instructions.push(axelar_solana_its::instruction::set_trusted_chain(
                    *fee_payer,
                    chain.clone(),
                )?);
            }
        } else {
            eyre::bail!("Failed to load all chains from chains info JSON file");
        }
    } else {
        instructions.push(axelar_solana_its::instruction::set_trusted_chain(
            *fee_payer,
            set_trusted_chain_args.chain_name,
        )?);
    }

    Ok(instructions)
}

fn remove_trusted_chain(
    fee_payer: &Pubkey,
    remove_trusted_chain_args: TrustedChainArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![axelar_solana_its::instruction::remove_trusted_chain(
        *fee_payer,
        remove_trusted_chain_args.chain_name,
    )?])
}

fn approve_deploy_remote_interchain_token(
    fee_payer: &Pubkey,
    args: ApproveDeployRemoteInterchainTokenArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    let destination_minter = decode_its_destination(
        &chains_info,
        &args.destination_chain,
        args.destination_minter,
    )?;

    Ok(vec![
        axelar_solana_its::instruction::approve_deploy_remote_interchain_token(
            *fee_payer,
            args.deployer,
            args.salt,
            args.destination_chain,
            destination_minter,
        )?,
    ])
}

fn revoke_deploy_remote_interchain_token(
    fee_payer: &Pubkey,
    args: RevokeDeployRemoteInterchainTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::revoke_deploy_remote_interchain_token(
            *fee_payer,
            args.deployer,
            args.salt,
            args.destination_chain,
        )?,
    ])
}

fn register_canonical_interchain_token(
    fee_payer: &Pubkey,
    args: RegisterCanonicalInterchainTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    let token_id = axelar_solana_its::canonical_interchain_token_id(&args.mint);
    println!("Token ID: {}", hex::encode(token_id));

    Ok(vec![
        axelar_solana_its::instruction::register_canonical_interchain_token(
            *fee_payer,
            args.mint,
            args.token_program,
        )?,
    ])
}

fn deploy_remote_canonical_interchain_token(
    fee_payer: &Pubkey,
    args: DeployRemoteCanonicalInterchainTokenArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    Ok(vec![
        axelar_solana_its::instruction::deploy_remote_canonical_interchain_token(
            *fee_payer,
            args.mint,
            args.destination_chain,
            args.gas_value,
            gas_service,
            gas_config_account,
        )?,
    ])
}

fn deploy_interchain_token(
    fee_payer: &Pubkey,
    args: DeployInterchainTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    let token_id = axelar_solana_its::interchain_token_id(fee_payer, &args.salt);
    println!("Token ID: {}", hex::encode(token_id));

    Ok(vec![
        axelar_solana_its::instruction::deploy_interchain_token(
            *fee_payer,
            args.salt,
            args.name,
            args.symbol,
            args.decimals,
            args.initial_supply,
            args.minter,
        )?,
    ])
}

fn deploy_remote_interchain_token(
    fee_payer: &Pubkey,
    args: DeployRemoteInterchainTokenArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    Ok(vec![
        axelar_solana_its::instruction::deploy_remote_interchain_token(
            *fee_payer,
            args.salt,
            args.destination_chain,
            args.gas_value,
            gas_service,
            gas_config_account,
        )?,
    ])
}

fn deploy_remote_interchain_token_with_minter(
    fee_payer: &Pubkey,
    args: DeployRemoteInterchainTokenWithMinterArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    let destination_minter = decode_its_destination(
        &chains_info,
        &args.destination_chain,
        args.destination_minter,
    )?;
    Ok(vec![
        axelar_solana_its::instruction::deploy_remote_interchain_token_with_minter(
            *fee_payer,
            args.salt,
            args.minter,
            args.destination_chain,
            destination_minter,
            args.gas_value,
            gas_service,
            gas_config_account,
        )?,
    ])
}

fn register_token_metadata(
    fee_payer: &Pubkey,
    args: RegisterTokenMetadataArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    Ok(vec![
        axelar_solana_its::instruction::register_token_metadata(
            *fee_payer,
            args.mint,
            args.token_program,
            args.gas_value,
            gas_service,
            gas_config_account,
        )?,
    ])
}

fn register_custom_token(
    fee_payer: &Pubkey,
    args: RegisterCustomTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    let token_id = axelar_solana_its::linked_token_id(fee_payer, &args.salt);
    println!("Token ID: {}", hex::encode(token_id));

    Ok(vec![axelar_solana_its::instruction::register_custom_token(
        *fee_payer,
        args.salt,
        args.mint,
        args.token_manager_type,
        args.token_program,
        args.operator,
    )?])
}

fn link_token(
    fee_payer: &Pubkey,
    args: LinkTokenArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;

    Ok(vec![axelar_solana_its::instruction::link_token(
        *fee_payer,
        args.salt,
        args.destination_chain,
        args.destination_token_address,
        args.token_manager_type,
        args.link_params,
        args.gas_value,
        gas_service,
        gas_config_account,
    )?])
}

fn interchain_transfer(
    fee_payer: &Pubkey,
    args: InterchainTransferArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    let timestamp: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        .try_into()?;

    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    let destination_address = decode_its_destination(
        &chains_info,
        &args.destination_chain,
        args.destination_address,
    )?;

    Ok(vec![axelar_solana_its::instruction::interchain_transfer(
        *fee_payer,
        args.source_account,
        args.authority,
        args.token_id,
        args.destination_chain,
        destination_address,
        args.amount,
        args.mint,
        args.token_program,
        args.gas_value,
        gas_service,
        gas_config_account,
        timestamp,
    )?])
}

fn call_contract_with_interchain_token(
    fee_payer: &Pubkey,
    args: CallContractWithInterchainTokenArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    let timestamp: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        .try_into()?;
    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    let destination_address = decode_its_destination(
        &chains_info,
        &args.destination_chain,
        args.destination_address,
    )?;
    Ok(vec![
        axelar_solana_its::instruction::call_contract_with_interchain_token(
            *fee_payer,
            args.source_account,
            args.authority,
            args.token_id,
            args.destination_chain,
            destination_address,
            args.amount,
            args.mint,
            args.data,
            args.token_program,
            args.gas_value,
            gas_service,
            gas_config_account,
            timestamp,
        )?,
    ])
}

fn call_contract_with_interchain_token_offchain_data(
    fee_payer: &Pubkey,
    args: CallContractWithInterchainTokenOffchainDataArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    let timestamp: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        .try_into()?;
    let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    let destination_address = decode_its_destination(
        &chains_info,
        &args.destination_chain,
        args.destination_address,
    )?;

    let (instruction, payload) =
        axelar_solana_its::instruction::call_contract_with_interchain_token_offchain_data(
            *fee_payer,
            args.source_account,
            args.authority,
            args.token_id,
            args.destination_chain,
            destination_address,
            args.amount,
            args.mint,
            args.data,
            args.token_program,
            args.gas_value,
            gas_service,
            gas_config_account,
            timestamp,
        )?;

    let mut file = File::create(config.output_dir.join("offchain_data_payload.bin"))?;
    file.write_all(&payload)?;

    Ok(vec![instruction])
}

fn set_flow_limit(fee_payer: &Pubkey, args: SetFlowLimitArgs) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![axelar_solana_its::instruction::set_flow_limit(
        *fee_payer,
        args.token_id,
        args.flow_limit,
    )?])
}

fn transfer_operatorship(
    fee_payer: &Pubkey,
    args: TransferOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![axelar_solana_its::instruction::transfer_operatorship(
        *fee_payer, args.to,
    )?])
}

fn propose_operatorship(
    fee_payer: &Pubkey,
    args: TransferOperatorshipArgs, // Reuses args from transfer
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![axelar_solana_its::instruction::propose_operatorship(
        *fee_payer, args.to,
    )?])
}

fn accept_operatorship(
    fee_payer: &Pubkey,
    args: AcceptOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![axelar_solana_its::instruction::accept_operatorship(
        *fee_payer, args.from,
    )?])
}

fn token_manager_set_flow_limit(
    fee_payer: &Pubkey,
    args: TokenManagerSetFlowLimitArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::token_manager::set_flow_limit(
            *fee_payer,
            args.token_id,
            args.flow_limit,
        )?,
    ])
}

fn token_manager_add_flow_limiter(
    fee_payer: &Pubkey,
    args: TokenManagerAddFlowLimiterArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::token_manager::add_flow_limiter(
            *fee_payer,
            args.token_id,
            args.flow_limiter,
        )?,
    ])
}

fn token_manager_remove_flow_limiter(
    fee_payer: &Pubkey,
    args: TokenManagerRemoveFlowLimiterArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::token_manager::remove_flow_limiter(
            *fee_payer,
            args.token_id,
            args.flow_limiter,
        )?,
    ])
}

fn token_manager_transfer_operatorship(
    fee_payer: &Pubkey,
    args: TokenManagerTransferOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::token_manager::transfer_operatorship(
            *fee_payer,
            args.token_id,
            args.to,
        )?,
    ])
}

fn token_manager_propose_operatorship(
    fee_payer: &Pubkey,
    args: TokenManagerProposeOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::token_manager::propose_operatorship(
            *fee_payer,
            args.token_id,
            args.to,
        )?,
    ])
}

fn token_manager_accept_operatorship(
    fee_payer: &Pubkey,
    args: TokenManagerAcceptOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::token_manager::accept_operatorship(
            *fee_payer,
            args.token_id,
            args.from,
        )?,
    ])
}

fn token_manager_handover_mint_authority(
    fee_payer: &Pubkey,
    args: TokenManagerHandoverMintAuthorityArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::token_manager::handover_mint_authority(
            *fee_payer,
            args.token_id,
            args.mint,
            args.token_program,
        )?,
    ])
}

fn interchain_token_mint(
    fee_payer: &Pubkey,
    args: InterchainTokenMintArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::interchain_token::mint(
            args.token_id,
            args.mint,
            args.to,
            *fee_payer, // Payer is the minter in this context
            args.token_program,
            args.amount,
        )?,
    ])
}

fn interchain_token_transfer_mintership(
    fee_payer: &Pubkey,
    args: InterchainTokenTransferMintershipArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::interchain_token::transfer_mintership(
            *fee_payer,
            args.token_id,
            args.to,
        )?,
    ])
}

fn interchain_token_propose_mintership(
    fee_payer: &Pubkey,
    args: InterchainTokenProposeMintershipArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::interchain_token::propose_mintership(
            *fee_payer,
            args.token_id,
            args.to,
        )?,
    ])
}

fn interchain_token_accept_mintership(
    fee_payer: &Pubkey,
    args: InterchainTokenAcceptMintershipArgs,
) -> eyre::Result<Vec<Instruction>> {
    Ok(vec![
        axelar_solana_its::instruction::interchain_token::accept_mintership(
            *fee_payer,
            args.token_id,
            args.from,
        )?,
    ])
}
