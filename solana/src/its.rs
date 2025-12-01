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
    UPGRADE_AUTHORITY_KEY, fetch_latest_blockhash, read_json_file_from_path,
    write_json_to_file_path,
};

const ITS_SEED: &[u8] = b"interchain-token-service";
const TOKEN_MANAGER_SEED: &[u8] = b"token-manager";
const INTERCHAIN_TOKEN_SEED: &[u8] = b"interchain-token";
const PREFIX_INTERCHAIN_TOKEN_ID: &[u8] = b"interchain-token-id";
const PREFIX_INTERCHAIN_TOKEN_SALT: &[u8] = b"interchain-token-salt";
const PREFIX_CANONICAL_TOKEN_SALT: &[u8] = b"canonical-token-salt";
const PREFIX_CUSTOM_TOKEN_SALT: &[u8] = b"solana-custom-token-salt";

#[derive(Debug, Clone, Copy, borsh::BorshDeserialize)]
#[repr(u8)]
enum TokenManagerType {
    NativeInterchainToken = 0,
    MintBurnFrom = 1,
    LockUnlock = 2,
    LockUnlockFee = 3,
    MintBurn = 4,
}

#[derive(borsh::BorshDeserialize, Debug)]
struct TokenManager {
    ty: TokenManagerType,
    _token_id: [u8; 32],
    token_address: Pubkey,
    _associated_token_account: Pubkey,
    flow_slot: FlowSlot,
    _bump: u8,
}

#[derive(borsh::BorshDeserialize, Debug)]
struct FlowSlot {
    flow_limit: Option<u64>,
    _flow_in: u64,
    _flow_out: u64,
    _epoch: u64,
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

fn get_chain_name_hash() -> [u8; 32] {
    solana_axelar_its::CHAIN_NAME_HASH
}

fn interchain_token_deployer_salt(deployer: &Pubkey, salt: &[u8; 32]) -> [u8; 32] {
    solana_sdk::keccak::hashv(&[
        PREFIX_INTERCHAIN_TOKEN_SALT,
        &get_chain_name_hash(),
        deployer.as_ref(),
        salt,
    ])
    .0
}

fn interchain_token_id_internal(salt: &[u8; 32]) -> [u8; 32] {
    solana_sdk::keccak::hashv(&[PREFIX_INTERCHAIN_TOKEN_ID, salt]).0
}

fn interchain_token_id(deployer: &Pubkey, salt: &[u8; 32]) -> [u8; 32] {
    let deploy_salt = interchain_token_deployer_salt(deployer, salt);
    interchain_token_id_internal(&deploy_salt)
}

fn canonical_interchain_token_deploy_salt(mint: &Pubkey) -> [u8; 32] {
    solana_sdk::keccak::hashv(&[
        PREFIX_CANONICAL_TOKEN_SALT,
        &get_chain_name_hash(),
        mint.as_ref(),
    ])
    .0
}

fn canonical_interchain_token_id(token_address: &Pubkey) -> [u8; 32] {
    let salt = canonical_interchain_token_deploy_salt(token_address);
    interchain_token_id_internal(&salt)
}

fn linked_token_deployer_salt(deployer: &Pubkey, salt: &[u8; 32]) -> [u8; 32] {
    solana_sdk::keccak::hashv(&[
        PREFIX_CUSTOM_TOKEN_SALT,
        &get_chain_name_hash(),
        deployer.as_ref(),
        salt,
    ])
    .0
}

fn linked_token_id(sender: &Pubkey, salt: &[u8; 32]) -> [u8; 32] {
    let deploy_salt = linked_token_deployer_salt(sender, salt);
    interchain_token_id_internal(&deploy_salt)
}

fn get_associated_token_address(
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
    token_program_id: &Pubkey,
) -> Pubkey {
    let associated_token_program_id = spl_associated_token_account_program_id();
    Pubkey::find_program_address(
        &[
            wallet_address.as_ref(),
            token_program_id.as_ref(),
            token_mint_address.as_ref(),
        ],
        &associated_token_program_id,
    )
    .0
}

fn spl_associated_token_account_program_id() -> Pubkey {
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
}

fn mpl_token_metadata_program_id() -> Pubkey {
    solana_sdk::pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s")
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

    /// Register a canonical token as an interchain token
    RegisterCanonicalInterchainToken(RegisterCanonicalInterchainTokenArgs),

    /// Deploy a canonical interchain token on a remote chain
    DeployRemoteCanonicalInterchainToken(DeployRemoteCanonicalInterchainTokenArgs),

    /// Deploy a new interchain token on Solana
    DeployInterchainToken(DeployInterchainTokenArgs),

    /// Deploy an existing interchain token to a remote chain
    DeployRemoteInterchainToken(DeployRemoteInterchainTokenArgs),

    /// Register token metadata with the Interchain Token Service Hub
    RegisterTokenMetadata(RegisterTokenMetadataArgs),

    /// Register a custom token with the Interchain Token Service
    RegisterCustomToken(RegisterCustomTokenArgs),

    /// Link a local token to a remote token
    LinkToken(LinkTokenArgs),

    /// Transfer interchain tokens
    InterchainTransfer(InterchainTransferArgs),

    /// Set the flow limit for an interchain token
    SetFlowLimit(SetFlowLimitArgs),

    /// Transfer the Interchain Token Service operatorship to another account
    TransferOperatorship(TransferOperatorshipArgs),

    /// Propose transfer of operatorship of the Interchain Token Service to another account
    ProposeOperatorship(ProposeOperatorshipArgs),

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

#[derive(Subcommand, Debug)]
pub(crate) enum InterchainTokenCommand {
    /// Mint interchain tokens (requires minter role)
    Mint(InterchainTokenMintArgs),
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTokenMintArgs {
    /// The token id of the Interchain Token
    #[clap(long, value_parser = parse_hex_bytes32)]
    token_id: [u8; 32],

    /// The wallet account that has minting authority
    #[clap(long)]
    minter: Pubkey,

    /// The token account to which the tokens will be minted
    #[clap(long)]
    to: Pubkey,

    /// The amount of tokens to mint
    #[clap(long)]
    amount: String,
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
    #[clap(long)]
    destination_token_address: String,

    /// The TokenManager type to use for this token
    #[clap(long, value_parser = parse_token_manager_type)]
    token_manager_type: TokenManagerType,

    /// Additional arguments for the link, depending on the chain specific implementation
    #[clap(long, default_value = "")]
    link_params: String,

    /// The amount of gas to pay for the cross-chain transaction
    #[clap(long)]
    gas_value: u64,

    /// The account that will link the token (defaults to fee payer)
    #[clap(long)]
    deployer: Option<Pubkey>,

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

    /// The account to transfer operatorship to
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct ProposeOperatorshipArgs {
    /// The account that proposes the operatorship transfer
    #[clap(long)]
    proposer: Pubkey,

    /// The account to propose operatorship transfer to
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
        "nativeinterchaintoken" | "native_interchain_token" => {
            Ok(TokenManagerType::NativeInterchainToken)
        }
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
    let mut data = &account.data[8..];
    let token_manager = TokenManager::deserialize(&mut data)?;
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
        Commands::RemoveTrustedChain(args) => remove_trusted_chain(fee_payer, args, config),
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
        Commands::RegisterTokenMetadata(args) => register_token_metadata(fee_payer, args),
        Commands::RegisterCustomToken(args) => register_custom_token(fee_payer, args, config),
        Commands::LinkToken(args) => link_token(fee_payer, args),
        Commands::InterchainTransfer(args) => interchain_transfer(fee_payer, args, config),
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
        },
        Commands::InterchainToken(command) => match command {
            InterchainTokenCommand::Mint(args) => interchain_token_mint(fee_payer, args, config),
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
    let program_data =
        solana_sdk::bpf_loader_upgradeable::get_program_data_address(&solana_axelar_its::id());

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
    }
    .data();

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
    let program_data =
        solana_sdk::bpf_loader_upgradeable::get_program_data_address(&solana_axelar_its::id());

    let ix_data = solana_axelar_its::instruction::SetPauseStatus {
        paused: set_pause_args.paused,
    }
    .data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts: vec![
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new_readonly(program_data, false),
            AccountMeta::new(its_root_pda, false),
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

    let rpc_client = RpcClient::new(config.url.clone());
    let (its_root_pda, _) = find_its_root_pda();

    let (user_roles_pda, _) = Pubkey::find_program_address(
        &[b"user-roles", its_root_pda.as_ref(), authority.as_ref()],
        &solana_axelar_its::id(),
    );

    let user_roles_account = if rpc_client.get_account(&user_roles_pda).is_ok() {
        user_roles_pda
    } else {
        solana_axelar_its::id()
    };

    if set_trusted_chain_args.chain_name == "all" {
        use borsh::BorshDeserialize;

        let its_account = rpc_client.get_account(&its_root_pda)?;

        let mut data = &*its_account.data;
        let _discriminator = <[u8; 8]>::deserialize(&mut data)?;
        let _its_hub_address = String::deserialize(&mut data)?;
        let _chain_name = String::deserialize(&mut data)?;
        let _paused = bool::deserialize(&mut data)?;
        let trusted_chains = Vec::<String>::deserialize(&mut data)?;

        let chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;

        if let serde_json::Value::Object(ref chains) = chains_info[CHAINS_KEY] {
            let mut skipped_count = 0;
            let mut added_count = 0;

            for chain in chains.keys() {
                if trusted_chains.contains(chain) {
                    println!("Skipping {chain} (already trusted)");
                    skipped_count += 1;
                    continue;
                }

                println!("\u{2713} Creating instruction to set {chain} as trusted on Solana ITS");
                added_count += 1;

                let ix_data = solana_axelar_its::instruction::SetTrustedChain {
                    chain_name: chain.clone(),
                }
                .data();

                let program_data = solana_sdk::bpf_loader_upgradeable::get_program_data_address(
                    &solana_axelar_its::id(),
                );

                let (event_authority, _) =
                    Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

                instructions.push(Instruction {
                    program_id: solana_axelar_its::id(),
                    accounts: vec![
                        AccountMeta::new(authority, true),
                        AccountMeta::new_readonly(user_roles_account, false),
                        AccountMeta::new_readonly(program_data, false),
                        AccountMeta::new(its_root_pda, false),
                        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
                        AccountMeta::new_readonly(event_authority, false),
                        AccountMeta::new_readonly(solana_axelar_its::id(), false),
                    ],
                    data: ix_data,
                });
            }

            println!("\nSummary:");
            println!("    - Chains to add: {added_count}");
            println!("    - Chains skipped: {skipped_count}");
            println!("    - Total chains: {}", chains.len());

            if added_count == 0 {
                println!("\nAll chains are already trusted.");
            }
        } else {
            eyre::bail!("Failed to load all chains from chains info JSON file");
        }
    } else {
        let ix_data = solana_axelar_its::instruction::SetTrustedChain {
            chain_name: set_trusted_chain_args.chain_name,
        }
        .data();

        let program_data =
            solana_sdk::bpf_loader_upgradeable::get_program_data_address(&solana_axelar_its::id());

        let (event_authority, _) =
            Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

        instructions.push(Instruction {
            program_id: solana_axelar_its::id(),
            accounts: vec![
                AccountMeta::new(authority, true),
                AccountMeta::new_readonly(user_roles_account, false),
                AccountMeta::new_readonly(program_data, false),
                AccountMeta::new(its_root_pda, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
                AccountMeta::new_readonly(event_authority, false),
                AccountMeta::new_readonly(solana_axelar_its::id(), false),
            ],
            data: ix_data,
        });
    }

    Ok(instructions)
}

fn remove_trusted_chain(
    fee_payer: &Pubkey,
    remove_trusted_chain_args: TrustedChainArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let authority = remove_trusted_chain_args.authority.unwrap_or(*fee_payer);
    let rpc_client = RpcClient::new(config.url.clone());
    let (its_root_pda, _) = find_its_root_pda();

    let (user_roles_pda, _) = Pubkey::find_program_address(
        &[b"user-roles", its_root_pda.as_ref(), authority.as_ref()],
        &solana_axelar_its::id(),
    );

    let user_roles_account = if rpc_client.get_account(&user_roles_pda).is_ok() {
        user_roles_pda
    } else {
        solana_axelar_its::id()
    };

    let ix_data = solana_axelar_its::instruction::RemoveTrustedChain {
        chain_name: remove_trusted_chain_args.chain_name,
    }
    .data();

    let program_data =
        solana_sdk::bpf_loader_upgradeable::get_program_data_address(&solana_axelar_its::id());

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts: vec![
            AccountMeta::new(authority, true),
            AccountMeta::new_readonly(user_roles_account, false),
            AccountMeta::new_readonly(program_data, false),
            AccountMeta::new(its_root_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(event_authority, false),
            AccountMeta::new_readonly(solana_axelar_its::id(), false),
        ],
        data: ix_data,
    }])
}

fn register_canonical_interchain_token(
    fee_payer: &Pubkey,
    args: RegisterCanonicalInterchainTokenArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let token_id = canonical_interchain_token_id(&args.mint);
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &token_id);

    let token_program = get_token_program_from_mint(&args.mint, config)?;
    let associated_token_program = spl_associated_token_account_program_id();
    let mpl_token_metadata_program = mpl_token_metadata_program_id();

    let (metadata_account, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            mpl_token_metadata_program.as_ref(),
            args.mint.as_ref(),
        ],
        &mpl_token_metadata_program,
    );

    let token_manager_ata =
        get_associated_token_address(&token_manager_pda, &args.mint, &token_program);

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    println!("------------------------------------------");
    println!("\u{1FA99} Token details:");
    println!();
    println!("- Interchain Token ID: {}", hex::encode(token_id));
    println!("- Mint Address: {}", args.mint);
    println!("- Token Manager: {token_manager_pda}");
    println!("- Token Manager ATA: {token_manager_ata}");
    println!("------------------------------------------");

    let accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(metadata_account, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(args.mint, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(associated_token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(solana_axelar_its::id(), false),
    ];

    let ix_data = solana_axelar_its::instruction::RegisterCanonicalInterchainToken {}.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn deploy_remote_canonical_interchain_token(
    fee_payer: &Pubkey,
    args: DeployRemoteCanonicalInterchainTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    let token_id = canonical_interchain_token_id(&args.mint);
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &token_id);

    let mpl_token_metadata_program = mpl_token_metadata_program_id();
    let (metadata_account, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            mpl_token_metadata_program.as_ref(),
            args.mint.as_ref(),
        ],
        &mpl_token_metadata_program,
    );

    let gateway_program = solana_axelar_gateway::id();
    let (gateway_root_pda, _) = Pubkey::find_program_address(&[b"gateway"], &gateway_program);

    let (call_contract_signing_pda, _) =
        Pubkey::find_program_address(&[b"gtw-call-contract"], &solana_axelar_its::id());

    let (gateway_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &gateway_program);

    let gas_service_program = args.gas_service.unwrap_or(solana_axelar_gas_service::id());
    let (gas_treasury, _) = Pubkey::find_program_address(&[b"gas-service"], &gas_service_program);

    let (gas_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &gas_service_program);

    println!("------------------------------------------");
    println!("\u{1FA99} Remote Canonical Deploy details:");
    println!();
    println!("- Interchain Token ID: {}", hex::encode(token_id));
    println!("- Mint Address: {}", args.mint);
    println!("- Destination Chain: {}", args.destination_chain);
    println!("------------------------------------------");

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    let accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(args.mint, false),
        AccountMeta::new_readonly(metadata_account, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(gateway_program, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(gateway_event_authority, false),
        AccountMeta::new(gas_treasury, false),
        AccountMeta::new_readonly(gas_service_program, false),
        AccountMeta::new_readonly(gas_event_authority, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(solana_axelar_its::id(), false),
    ];

    let ix_data = solana_axelar_its::instruction::DeployRemoteCanonicalInterchainToken {
        destination_chain: args.destination_chain,
        gas_value: args.gas_value,
    }
    .data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn deploy_interchain_token(
    fee_payer: &Pubkey,
    args: DeployInterchainTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    let raw_supply =
        crate::utils::parse_decimal_string_to_raw_units(&args.initial_supply, args.decimals)?;

    let deployer = args.deployer.unwrap_or(*fee_payer);
    let token_id = interchain_token_id(&deployer, &args.salt);
    let (its_root_pda, _) = find_its_root_pda();
    let (mint, _) = find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &token_id);

    println!("------------------------------------------");
    println!("\u{1FA99} Token details:");
    println!();
    println!("- Interchain Token ID: {}", hex::encode(token_id));
    println!("- Mint Address: {mint}");
    println!("- Token Manager: {token_manager_pda}");
    println!("- Amount: {} {}", args.initial_supply, args.symbol);
    println!("- Decimals: {}", args.decimals);
    println!("------------------------------------------");

    let token_program = spl_token_2022::id();
    let associated_token_program = spl_associated_token_account_program_id();
    let mpl_token_metadata_program = mpl_token_metadata_program_id();

    let deployer_ata = get_associated_token_address(&deployer, &mint, &token_program);

    let token_manager_ata = get_associated_token_address(&token_manager_pda, &mint, &token_program);

    let (mpl_token_metadata_account, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            mpl_token_metadata_program.as_ref(),
            mint.as_ref(),
        ],
        &mpl_token_metadata_program,
    );

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    let mut accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(deployer, true),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(associated_token_program, false),
        AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
        AccountMeta::new_readonly(mpl_token_metadata_program, false),
        AccountMeta::new(mpl_token_metadata_account, false),
        AccountMeta::new(deployer_ata, false),
    ];

    let (minter_account, minter_roles_pda) = if let Some(ref minter) = args.minter {
        let (minter_roles, _) = Pubkey::find_program_address(
            &[b"user-roles", token_manager_pda.as_ref(), minter.as_ref()],
            &solana_axelar_its::id(),
        );
        (*minter, minter_roles)
    } else {
        (solana_axelar_its::id(), solana_axelar_its::id())
    };

    accounts.push(AccountMeta::new_readonly(minter_account, false));
    accounts.push(AccountMeta::new(minter_roles_pda, false));
    accounts.push(AccountMeta::new_readonly(event_authority, false));
    accounts.push(AccountMeta::new_readonly(solana_axelar_its::id(), false));

    let ix_data = solana_axelar_its::instruction::DeployInterchainToken {
        salt: args.salt,
        name: args.name,
        symbol: args.symbol,
        decimals: args.decimals,
        initial_supply: raw_supply,
    }
    .data();

    if args.minter.is_some() {
        println!("- Minter Roles PDA: {minter_roles_pda}");
    }

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn deploy_remote_interchain_token(
    fee_payer: &Pubkey,
    args: DeployRemoteInterchainTokenArgs,
) -> eyre::Result<Vec<Instruction>> {
    let deployer = args.deployer.unwrap_or(*fee_payer);
    let token_id = interchain_token_id(&deployer, &args.salt);
    let (its_root_pda, _) = find_its_root_pda();
    let (mint, _) = find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &token_id);

    let mpl_token_metadata_program = mpl_token_metadata_program_id();
    let (metadata_account, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            mpl_token_metadata_program.as_ref(),
            mint.as_ref(),
        ],
        &mpl_token_metadata_program,
    );

    let gateway_program = solana_axelar_gateway::id();
    let (gateway_root_pda, _) = Pubkey::find_program_address(&[b"gateway"], &gateway_program);

    let (call_contract_signing_pda, _) =
        Pubkey::find_program_address(&[b"gtw-call-contract"], &solana_axelar_its::id());

    let (gateway_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &gateway_program);

    let gas_service_program = args.gas_service.unwrap_or(solana_axelar_gas_service::id());
    let (gas_treasury, _) = Pubkey::find_program_address(&[b"gas-service"], &gas_service_program);

    let (gas_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &gas_service_program);

    println!("------------------------------------------");
    println!("\u{1FA99} Remote Deploy details:");
    println!();
    println!("- Interchain Token ID: {}", hex::encode(token_id));
    println!("- Mint Address: {mint}");
    println!("- Destination Chain: {}", args.destination_chain);
    println!("------------------------------------------");

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    let accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(deployer, true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(metadata_account, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(gateway_program, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(gateway_event_authority, false),
        AccountMeta::new(gas_treasury, false),
        AccountMeta::new_readonly(gas_service_program, false),
        AccountMeta::new_readonly(gas_event_authority, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(solana_axelar_its::id(), false),
    ];

    let ix_data = solana_axelar_its::instruction::DeployRemoteInterchainToken {
        salt: args.salt,
        destination_chain: args.destination_chain,
        gas_value: args.gas_value,
    }
    .data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn register_token_metadata(
    fee_payer: &Pubkey,
    args: RegisterTokenMetadataArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (its_root_pda, _) = find_its_root_pda();

    let gateway_program = solana_axelar_gateway::id();
    let (gateway_root_pda, _) = Pubkey::find_program_address(&[b"gateway"], &gateway_program);

    let (call_contract_signing_pda, _) =
        Pubkey::find_program_address(&[b"gtw-call-contract"], &solana_axelar_its::id());

    let (gateway_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &gateway_program);

    let gas_service_program = args.gas_service.unwrap_or(solana_axelar_gas_service::id());
    let (gas_treasury, _) = Pubkey::find_program_address(&[b"gas-service"], &gas_service_program);

    let (gas_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &gas_service_program);

    println!("------------------------------------------");
    println!("\u{1FA99} Register Token Metadata:");
    println!();
    println!("- Mint Address: {}", args.mint);
    println!("- Gas Value: {}", args.gas_value);
    println!("------------------------------------------");

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    let accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(args.mint, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(gateway_program, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(gateway_event_authority, false),
        AccountMeta::new(gas_treasury, false),
        AccountMeta::new_readonly(gas_service_program, false),
        AccountMeta::new_readonly(gas_event_authority, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(solana_axelar_its::id(), false),
    ];

    let ix_data = solana_axelar_its::instruction::RegisterTokenMetadata {
        gas_value: args.gas_value,
    }
    .data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn register_custom_token(
    fee_payer: &Pubkey,
    args: RegisterCustomTokenArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let deployer = args.deployer.unwrap_or(*fee_payer);
    let token_id = linked_token_id(&deployer, &args.salt);
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &token_id);

    let token_program = get_token_program_from_mint(&args.mint, config)?;
    let associated_token_program = spl_associated_token_account_program_id();

    let token_manager_ata =
        get_associated_token_address(&token_manager_pda, &args.mint, &token_program);

    println!("------------------------------------------");
    println!("\u{1FA99} Token details:");
    println!();
    println!("- Interchain Token ID: {}", hex::encode(token_id));
    println!("- Mint Address: {}", args.mint);
    println!("- Token Manager: {token_manager_pda}");
    println!("- Token Manager Type: {:#?}", args.token_manager_type);
    println!("------------------------------------------");

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    let mut accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(deployer, true),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(args.mint, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(associated_token_program, false),
    ];

    let (operator_account, operator_roles_pda) = if let Some(ref operator) = args.operator {
        let (operator_roles, _) = Pubkey::find_program_address(
            &[b"user-roles", token_manager_pda.as_ref(), operator.as_ref()],
            &solana_axelar_its::id(),
        );
        (*operator, operator_roles)
    } else {
        (solana_axelar_its::id(), solana_axelar_its::id())
    };

    accounts.push(AccountMeta::new_readonly(operator_account, false));
    accounts.push(AccountMeta::new(operator_roles_pda, false));
    accounts.push(AccountMeta::new_readonly(event_authority, false));
    accounts.push(AccountMeta::new_readonly(solana_axelar_its::id(), false));

    let token_manager_type = match args.token_manager_type {
        TokenManagerType::NativeInterchainToken => {
            solana_axelar_its::state::Type::NativeInterchainToken
        }
        TokenManagerType::MintBurnFrom => solana_axelar_its::state::Type::MintBurnFrom,
        TokenManagerType::LockUnlock => solana_axelar_its::state::Type::LockUnlock,
        TokenManagerType::LockUnlockFee => solana_axelar_its::state::Type::LockUnlockFee,
        TokenManagerType::MintBurn => solana_axelar_its::state::Type::MintBurn,
    };

    let ix_data = solana_axelar_its::instruction::RegisterCustomToken {
        salt: args.salt,
        token_manager_type,
        operator: args.operator,
    }
    .data();

    if args.operator.is_some() {
        println!("- Operator Roles PDA: {operator_roles_pda}");
    }

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn link_token(fee_payer: &Pubkey, args: LinkTokenArgs) -> eyre::Result<Vec<Instruction>> {
    let deployer = args.deployer.unwrap_or(*fee_payer);
    let token_id = linked_token_id(&deployer, &args.salt);
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &token_id);

    let gateway_program = solana_axelar_gateway::id();
    let (gateway_root_pda, _) = Pubkey::find_program_address(&[b"gateway"], &gateway_program);

    let (call_contract_signing_pda, _) =
        Pubkey::find_program_address(&[b"gtw-call-contract"], &solana_axelar_its::id());

    let (gateway_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &gateway_program);

    let gas_service_program = args.gas_service.unwrap_or(solana_axelar_gas_service::id());
    let (gas_treasury, _) = Pubkey::find_program_address(&[b"gas-service"], &gas_service_program);

    let (gas_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &gas_service_program);

    let destination_token_address =
        hex::decode(args.destination_token_address.trim_start_matches("0x"))?;

    println!("------------------------------------------");
    println!("\u{1FA99} Link Token details:");
    println!();
    println!("- Interchain Token ID: {}", hex::encode(token_id));
    println!("- Token Manager: {token_manager_pda}");
    println!("- Destination Chain: {}", args.destination_chain);
    println!(
        "- Destination Token: {}",
        hex::encode(&destination_token_address)
    );
    println!("------------------------------------------");

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    let accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(deployer, true),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(gateway_program, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(gateway_event_authority, false),
        AccountMeta::new(gas_treasury, false),
        AccountMeta::new_readonly(gas_service_program, false),
        AccountMeta::new_readonly(gas_event_authority, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(solana_axelar_its::id(), false),
    ];

    let token_manager_type = match args.token_manager_type {
        TokenManagerType::NativeInterchainToken => {
            solana_axelar_its::state::Type::NativeInterchainToken
        }
        TokenManagerType::MintBurnFrom => solana_axelar_its::state::Type::MintBurnFrom,
        TokenManagerType::LockUnlock => solana_axelar_its::state::Type::LockUnlock,
        TokenManagerType::LockUnlockFee => solana_axelar_its::state::Type::LockUnlockFee,
        TokenManagerType::MintBurn => solana_axelar_its::state::Type::MintBurn,
    };

    let link_params = if args.link_params.is_empty() {
        vec![]
    } else {
        hex::decode(args.link_params.trim_start_matches("0x"))?
    };

    let ix_data = solana_axelar_its::instruction::LinkToken {
        salt: args.salt,
        destination_chain: args.destination_chain,
        destination_token_address,
        token_manager_type,
        link_params,
        gas_value: args.gas_value,
    }
    .data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn interchain_transfer(
    fee_payer: &Pubkey,
    args: InterchainTransferArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mint = get_mint_from_token_manager(&args.token_id, config)?;
    let decimals = get_token_decimals(&mint, config)?;
    let raw_amount = crate::utils::parse_decimal_string_to_raw_units(&args.amount, decimals)?;

    let authority = args.authority.unwrap_or(*fee_payer);
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &args.token_id);

    let token_program = get_token_program_from_mint(&mint, config)?;
    let token_manager_ata = get_associated_token_address(&token_manager_pda, &mint, &token_program);

    let gateway_program = solana_axelar_gateway::id();
    let (gateway_root_pda, _) = Pubkey::find_program_address(&[b"gateway"], &gateway_program);

    let (call_contract_signing_pda, _) =
        Pubkey::find_program_address(&[b"gtw-call-contract"], &solana_axelar_its::id());

    let (gateway_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &gateway_program);

    let gas_service_program = args.gas_service.unwrap_or(solana_axelar_gas_service::id());
    let (gas_treasury, _) = Pubkey::find_program_address(&[b"gas-service"], &gas_service_program);

    let (gas_event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &gas_service_program);

    println!("------------------------------------------");
    println!("\u{1FA99} Transfer details:");
    println!();
    println!("- Amount: {} tokens", args.amount);
    println!("- Raw Amount: {raw_amount}");
    println!("- Decimals: {decimals}");
    println!("- Destination Chain: {}", args.destination_chain);
    println!("- Destination Address: {}", args.destination_address);
    println!("------------------------------------------");

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    let accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(gateway_event_authority, false),
        AccountMeta::new_readonly(gateway_program, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new(gas_treasury, false),
        AccountMeta::new_readonly(gas_service_program, false),
        AccountMeta::new_readonly(gas_event_authority, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(args.source_account, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(solana_axelar_its::id(), false),
    ];

    let ix_data = solana_axelar_its::instruction::InterchainTransfer {
        token_id: args.token_id,
        destination_chain: args.destination_chain,
        destination_address: args.destination_address.into_bytes(),
        amount: raw_amount,
        gas_value: args.gas_value,
        source_id: None,
        pda_seeds: None,
        data: None,
    }
    .data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn set_flow_limit(_fee_payer: &Pubkey, args: SetFlowLimitArgs) -> eyre::Result<Vec<Instruction>> {
    let operator = args.operator.unwrap_or(*_fee_payer);
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &args.token_id);

    let (its_roles_pda, _) = Pubkey::find_program_address(
        &[b"user-roles", its_root_pda.as_ref(), operator.as_ref()],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Set Flow Limit (ITS-level):");
    println!();
    println!("- Token ID: {}", hex::encode(args.token_id));
    println!("- Flow Limit: {:?}", args.flow_limit);
    println!("- Operator: {operator}");
    println!("------------------------------------------");

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    let accounts = vec![
        AccountMeta::new(*_fee_payer, true),
        AccountMeta::new_readonly(operator, true),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(its_roles_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(solana_axelar_its::id(), false),
    ];

    let ix_data = solana_axelar_its::instruction::SetFlowLimit {
        flow_limit: Some(args.flow_limit),
    }
    .data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn transfer_operatorship(
    fee_payer: &Pubkey,
    args: TransferOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (its_root_pda, _) = find_its_root_pda();

    let (origin_roles_pda, _) = Pubkey::find_program_address(
        &[b"user-roles", its_root_pda.as_ref(), args.sender.as_ref()],
        &solana_axelar_its::id(),
    );

    let (destination_roles_pda, _) = Pubkey::find_program_address(
        &[b"user-roles", its_root_pda.as_ref(), args.to.as_ref()],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Transfer Operatorship (ITS):");
    println!();
    println!("- From: {}", args.sender);
    println!("- To: {}", args.to);
    println!("------------------------------------------");

    let accounts = vec![
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(args.sender, true),
        AccountMeta::new(origin_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(args.to, false),
        AccountMeta::new(destination_roles_pda, false),
    ];

    let ix_data = solana_axelar_its::instruction::TransferOperatorship {}.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn propose_operatorship(
    fee_payer: &Pubkey,
    args: ProposeOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (its_root_pda, _) = find_its_root_pda();

    let (origin_roles_pda, _) = Pubkey::find_program_address(
        &[b"user-roles", its_root_pda.as_ref(), args.proposer.as_ref()],
        &solana_axelar_its::id(),
    );

    let (proposal_pda, _) = Pubkey::find_program_address(
        &[
            b"role-proposal",
            its_root_pda.as_ref(),
            args.proposer.as_ref(),
            args.to.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Propose Operatorship (ITS):");
    println!();
    println!("- Proposer: {}", args.proposer);
    println!("- To: {}", args.to);
    println!("- Proposal PDA: {proposal_pda}");
    println!("------------------------------------------");

    let accounts = vec![
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(args.proposer, true),
        AccountMeta::new(origin_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(args.to, false),
        AccountMeta::new(proposal_pda, false),
    ];

    let ix_data = solana_axelar_its::instruction::ProposeOperatorship {}.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn accept_operatorship(
    fee_payer: &Pubkey,
    args: AcceptOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (its_root_pda, _) = find_its_root_pda();

    let (destination_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            its_root_pda.as_ref(),
            args.role_receiver.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    let (origin_roles_pda, _) = Pubkey::find_program_address(
        &[b"user-roles", its_root_pda.as_ref(), args.from.as_ref()],
        &solana_axelar_its::id(),
    );

    let (proposal_pda, _) = Pubkey::find_program_address(
        &[
            b"role-proposal",
            its_root_pda.as_ref(),
            args.from.as_ref(),
            args.role_receiver.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Accept Operatorship (ITS):");
    println!();
    println!("- Role Receiver: {}", args.role_receiver);
    println!("- From: {}", args.from);
    println!("------------------------------------------");

    let accounts = vec![
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(args.role_receiver, true),
        AccountMeta::new(destination_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(args.from, false),
        AccountMeta::new(origin_roles_pda, false),
        AccountMeta::new(proposal_pda, false),
    ];

    let ix_data = solana_axelar_its::instruction::AcceptOperatorship {}.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn token_manager_set_flow_limit(
    fee_payer: &Pubkey,
    args: TokenManagerSetFlowLimitArgs,
) -> eyre::Result<Vec<Instruction>> {
    let flow_limiter = args.operator.unwrap_or(*fee_payer);
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &args.token_id);

    let (flow_limiter_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            token_manager_pda.as_ref(),
            flow_limiter.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Set Token Manager Flow Limit:");
    println!();
    println!("- Token ID: {}", hex::encode(args.token_id));
    println!("- Flow Limit: {}", args.flow_limit);
    println!("- Flow Limiter: {flow_limiter}");
    println!("------------------------------------------");

    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_its::id());

    let accounts = vec![
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(flow_limiter, true),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(flow_limiter_roles_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(solana_axelar_its::id(), false),
    ];

    let ix_data = solana_axelar_its::instruction::SetTokenManagerFlowLimit {
        flow_limit: Some(args.flow_limit),
    }
    .data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn token_manager_add_flow_limiter(
    fee_payer: &Pubkey,
    args: TokenManagerAddFlowLimiterArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &args.token_id);

    let (authority_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            token_manager_pda.as_ref(),
            args.adder.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    let (target_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            token_manager_pda.as_ref(),
            args.flow_limiter.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Add Flow Limiter:");
    println!();
    println!("- Token ID: {}", hex::encode(args.token_id));
    println!("- Adder: {}", args.adder);
    println!("- Flow Limiter: {}", args.flow_limiter);
    println!("------------------------------------------");

    let accounts = vec![
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(args.adder, true),
        AccountMeta::new_readonly(authority_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(args.flow_limiter, false),
        AccountMeta::new(target_roles_pda, false),
    ];

    let ix_data = solana_axelar_its::instruction::AddTokenManagerFlowLimiter {}.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn token_manager_remove_flow_limiter(
    fee_payer: &Pubkey,
    args: TokenManagerRemoveFlowLimiterArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &args.token_id);

    let (authority_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            token_manager_pda.as_ref(),
            args.remover.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    let (target_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            token_manager_pda.as_ref(),
            args.flow_limiter.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Remove Flow Limiter:");
    println!();
    println!("- Token ID: {}", hex::encode(args.token_id));
    println!("- Remover: {}", args.remover);
    println!("- Flow Limiter: {}", args.flow_limiter);
    println!("------------------------------------------");

    let accounts = vec![
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(args.remover, true),
        AccountMeta::new_readonly(authority_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(args.flow_limiter, false),
        AccountMeta::new(target_roles_pda, false),
    ];

    let ix_data = solana_axelar_its::instruction::RemoveTokenManagerFlowLimiter {}.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn token_manager_transfer_operatorship(
    fee_payer: &Pubkey,
    args: TokenManagerTransferOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &args.token_id);

    let (origin_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            token_manager_pda.as_ref(),
            args.sender.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    let (destination_roles_pda, _) = Pubkey::find_program_address(
        &[b"user-roles", token_manager_pda.as_ref(), args.to.as_ref()],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Transfer Token Manager Operatorship:");
    println!();
    println!("- Token ID: {}", hex::encode(args.token_id));
    println!("- From: {}", args.sender);
    println!("- To: {}", args.to);
    println!("------------------------------------------");

    let accounts = vec![
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(args.sender, true),
        AccountMeta::new(origin_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(args.to, false),
        AccountMeta::new(destination_roles_pda, false),
    ];

    let ix_data = solana_axelar_its::instruction::TransferTokenManagerOperatorship {}.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn token_manager_propose_operatorship(
    fee_payer: &Pubkey,
    args: TokenManagerProposeOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &args.token_id);

    let (origin_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            token_manager_pda.as_ref(),
            args.proposer.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    let (proposal_pda, _) = Pubkey::find_program_address(
        &[
            b"role-proposal",
            token_manager_pda.as_ref(),
            args.proposer.as_ref(),
            args.to.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Propose Token Manager Operatorship:");
    println!();
    println!("- Token ID: {}", hex::encode(args.token_id));
    println!("- Proposer: {}", args.proposer);
    println!("- To: {}", args.to);
    println!("------------------------------------------");

    let accounts = vec![
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(args.proposer, true),
        AccountMeta::new(origin_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(args.to, false),
        AccountMeta::new(proposal_pda, false),
    ];

    let ix_data = solana_axelar_its::instruction::ProposeTokenManagerOperatorship {}.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn token_manager_accept_operatorship(
    fee_payer: &Pubkey,
    args: TokenManagerAcceptOperatorshipArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &args.token_id);

    let (destination_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            token_manager_pda.as_ref(),
            args.accepter.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    let (origin_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            token_manager_pda.as_ref(),
            args.from.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    let (proposal_pda, _) = Pubkey::find_program_address(
        &[
            b"role-proposal",
            token_manager_pda.as_ref(),
            args.from.as_ref(),
            args.accepter.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Accept Token Manager Operatorship:");
    println!();
    println!("- Token ID: {}", hex::encode(args.token_id));
    println!("- Accepter: {}", args.accepter);
    println!("- From: {}", args.from);
    println!("------------------------------------------");

    let accounts = vec![
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(args.accepter, true),
        AccountMeta::new(destination_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new(args.from, false),
        AccountMeta::new(origin_roles_pda, false),
        AccountMeta::new(proposal_pda, false),
    ];

    let ix_data = solana_axelar_its::instruction::AcceptTokenManagerOperatorship {}.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
}

fn interchain_token_mint(
    _fee_payer: &Pubkey,
    args: InterchainTokenMintArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let mint = get_mint_from_token_manager(&args.token_id, config)?;
    let decimals = get_token_decimals(&mint, config)?;
    let raw_amount = crate::utils::parse_decimal_string_to_raw_units(&args.amount, decimals)?;

    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &args.token_id);

    let token_program = get_token_program_from_mint(&mint, config)?;

    let rpc_client = RpcClient::new(config.url.clone());
    let destination_account_info = rpc_client.get_account(&args.to)?;
    if destination_account_info.owner != token_program {
        eyre::bail!("Destination account is not owned by the correct token program");
    }

    let (minter_roles_pda, _) = Pubkey::find_program_address(
        &[
            b"user-roles",
            token_manager_pda.as_ref(),
            args.minter.as_ref(),
        ],
        &solana_axelar_its::id(),
    );

    println!("------------------------------------------");
    println!("\u{1FA99} Mint details:");
    println!();
    println!("- Token ID: {}", hex::encode(args.token_id));
    println!("- Minter: {}", args.minter);
    println!("- Destination: {}", args.to);
    println!("- Amount: {}", args.amount);
    println!("- Raw Amount: {raw_amount}");
    println!("------------------------------------------");

    let accounts = vec![
        AccountMeta::new(mint, false),
        AccountMeta::new(args.to, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(args.minter, true),
        AccountMeta::new_readonly(minter_roles_pda, false),
        AccountMeta::new_readonly(token_program, false),
    ];

    let ix_data = solana_axelar_its::instruction::MintInterchainToken { amount: raw_amount }.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_its::id(),
        accounts,
        data: ix_data,
    }])
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
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, &token_id);
    let account = rpc_client.get_account(&token_manager_pda)?;
    let mut data = &account.data[8..];
    let token_manager = TokenManager::deserialize(&mut data)?;

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
