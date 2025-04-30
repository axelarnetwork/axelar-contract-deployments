use std::{fs::File, io::Write, time::SystemTime};

use axelar_solana_its::state;
use clap::{Parser, Subcommand};
use serde::Deserialize;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;

use crate::config::Config;
use crate::types::ChainNameOnAxelar;
use crate::utils::{
    self, ADDRESS_KEY, AXELAR_KEY, CONTRACTS_KEY, GAS_CONFIG_ACCOUNT, GAS_SERVICE_KEY, ITS_KEY,
};

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    #[clap(long_about = "Initialize the ITS program")]
    Init(InitArgs),

    #[clap(long_about = "Set the pause status of the ITS program")]
    SetPauseStatus(SetPauseStatusArgs),

    #[clap(long_about = "Add a new trusted chain to ITS")]
    SetTrustedChain(TrustedChainArgs),

    #[clap(long_about = "Remove an existing trusted chain from ITS")]
    RemoveTrustedChain(TrustedChainArgs),

    #[clap(long_about = "Approve deploying a remote interchain token with a specific minter")]
    ApproveDeployRemoteInterchainToken(ApproveDeployRemoteInterchainTokenArgs),

    #[clap(long_about = "Revoke approval for deploying a remote interchain token")]
    RevokeDeployRemoteInterchainToken(RevokeDeployRemoteInterchainTokenArgs),

    #[clap(long_about = "Register a canonical token as an interchain token")]
    RegisterCanonicalInterchainToken(RegisterCanonicalInterchainTokenArgs),

    #[clap(long_about = "Deploy a canonical interchain token on a remote chain")]
    DeployRemoteCanonicalInterchainToken(DeployRemoteCanonicalInterchainTokenArgs),

    #[clap(long_about = "Deploy a new interchain token")]
    DeployInterchainToken(DeployInterchainTokenArgs),

    #[clap(long_about = "Deploy an existing interchain token to a remote chain")]
    DeployRemoteInterchainToken(DeployRemoteInterchainTokenArgs),

    #[clap(
        long_about = "Deploy an existing interchain token to a remote chain with a specific minter"
    )]
    DeployRemoteInterchainTokenWithMinter(DeployRemoteInterchainTokenWithMinterArgs),

    #[clap(long_about = "Register token metadata with the ITS hub")]
    RegisterTokenMetadata(RegisterTokenMetadataArgs),

    #[clap(long_about = "Register a custom token with ITS")]
    RegisterCustomToken(RegisterCustomTokenArgs),

    #[clap(long_about = "Link a local token to a remote token")]
    LinkToken(LinkTokenArgs),

    #[clap(long_about = "Transfer interchain tokens to a remote chain")]
    InterchainTransfer(InterchainTransferArgs),

    #[clap(long_about = "Transfer interchain tokens and call a contract on the remote chain")]
    CallContractWithInterchainToken(CallContractWithInterchainTokenArgs),

    #[clap(
        long_about = "Transfer interchain tokens and call a contract on the remote chain using offchain data"
    )]
    CallContractWithInterchainTokenOffchainData(CallContractWithInterchainTokenOffchainDataArgs),

    #[clap(long_about = "Set the flow limit for a token manager")]
    SetFlowLimit(SetFlowLimitArgs),

    #[clap(long_about = "Transfer ITS operatorship")]
    TransferOperatorship(TransferOperatorshipArgs),

    #[clap(long_about = "Propose ITS operatorship transfer")]
    ProposeOperatorship(TransferOperatorshipArgs), // Uses same args as transfer

    #[clap(long_about = "Accept ITS operatorship transfer")]
    AcceptOperatorship(AcceptOperatorshipArgs),
}

// Helper functions for parsing CLI arguments
fn hash_salt(s: &str) -> eyre::Result<[u8; 32]> {
    Ok(solana_sdk::keccak::hash(s.as_bytes()).0)
}

fn parse_hex_vec(s: &str) -> Result<Vec<u8>, hex::FromHexError> {
    hex::decode(s.strip_prefix("0x").unwrap_or(s))
}

fn parse_token_program(s: &str) -> Result<Pubkey, String> {
    match s.to_lowercase().as_str() {
        "spl_token" => Ok(spl_token::id()),
        "spl_token_2022" => Ok(spl_token_2022::id()),
        _ => Err(format!("Invalid token program: {}", s)),
    }
}

fn parse_token_manager_type(s: &str) -> Result<state::token_manager::Type, String> {
    match s.to_lowercase().as_str() {
        "lockunlock" | "lock_unlock" => Ok(state::token_manager::Type::LockUnlock),
        "mintburn" | "mint_burn" => Ok(state::token_manager::Type::MintBurn),
        "mintburnfrom" | "mint_burn_from" => Ok(state::token_manager::Type::MintBurnFrom),
        "lockunlockfee" | "lock_unlock_fee" => Ok(state::token_manager::Type::LockUnlockFee),
        _ => Err(format!("Invalid token manager type: {}", s)),
    }
}

fn try_infer_gas_service_id(maybe_arg: Option<Pubkey>, config: &Config) -> eyre::Result<Pubkey> {
    match maybe_arg {
        Some(id) => Ok(id),
        None => {
            let id = Pubkey::deserialize(
                &utils::chains_info(config.network_type)
                    [ChainNameOnAxelar::from(config.network_type).0][CONTRACTS_KEY]
                    [GAS_SERVICE_KEY][ADDRESS_KEY],
            ).map_err(|_| eyre::eyre!(
                "Could not get the gas service id from the chains info JSON file. Is it already deployed? \
                Please update the file or pass a value to --gas-service"))?;

            Ok(id)
        }
    }
}

fn try_infer_gas_service_config_account(
    maybe_arg: Option<Pubkey>,
    config: &Config,
) -> eyre::Result<Pubkey> {
    match maybe_arg {
        Some(id) => Ok(id),
        None => {
            let id = Pubkey::deserialize(
                &utils::chains_info(config.network_type)
                    [ChainNameOnAxelar::from(config.network_type).0][CONTRACTS_KEY]
                    [GAS_SERVICE_KEY][GAS_CONFIG_ACCOUNT],
            ).map_err(|_| eyre::eyre!(
                "Could not get the gas service config PDA from the chains info JSON file. Is it already deployed? \
                Please update the file or pass a value to --gas-config-account"))?;

            Ok(id)
        }
    }
}

#[derive(Parser, Debug)]
pub(crate) struct InitArgs {
    #[clap(short, long)]
    operator: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct SetPauseStatusArgs {
    #[clap(short, long)]
    paused: bool,
}

#[derive(Parser, Debug)]
pub(crate) struct TrustedChainArgs {
    #[clap(short, long)]
    chain_name: String,
}

#[derive(Parser, Debug)]
pub(crate) struct ApproveDeployRemoteInterchainTokenArgs {
    #[clap(long)]
    deployer: Pubkey,
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],
    #[clap(long)]
    destination_chain: String,
    #[clap(long)]
    destination_minter: String,
}

#[derive(Parser, Debug)]
pub(crate) struct RevokeDeployRemoteInterchainTokenArgs {
    #[clap(long)]
    deployer: Pubkey,
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],
    #[clap(long)]
    destination_chain: String,
}

#[derive(Parser, Debug)]
pub(crate) struct RegisterCanonicalInterchainTokenArgs {
    #[clap(long)]
    mint: Pubkey,

    /// The token program to use for the mint. This can be either spl_token or spl_token_2022.
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct DeployRemoteCanonicalInterchainTokenArgs {
    #[clap(long)]
    mint: Pubkey,
    #[clap(long)]
    destination_chain: String,
    #[clap(long)]
    gas_value: u64,
    #[clap(long)]
    gas_service: Option<Pubkey>,
    #[clap(long)]
    gas_config_account: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct DeployInterchainTokenArgs {
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],
    #[clap(long)]
    name: String,
    #[clap(long)]
    symbol: String,
    #[clap(long)]
    decimals: u8,
    #[clap(long)]
    initial_supply: u64,
    #[clap(long)]
    minter: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct DeployRemoteInterchainTokenArgs {
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],
    #[clap(long)]
    destination_chain: String,
    #[clap(long)]
    gas_value: u64,
    #[clap(long)]
    gas_service: Option<Pubkey>,
    #[clap(long)]
    gas_config_account: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct DeployRemoteInterchainTokenWithMinterArgs {
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],
    #[clap(long)]
    minter: Pubkey,
    #[clap(long)]
    destination_chain: String,
    #[clap(long)]
    destination_minter: String,
    #[clap(long)]
    gas_value: u64,
    #[clap(long)]
    gas_service: Option<Pubkey>,
    #[clap(long)]
    gas_config_account: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct RegisterTokenMetadataArgs {
    #[clap(long)]
    mint: Pubkey,

    /// The token program to use for the mint. This can be either spl_token or spl_token_2022.
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,
    #[clap(long)]
    gas_value: u64,
    #[clap(long)]
    gas_service: Option<Pubkey>,
    #[clap(long)]
    gas_config_account: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct RegisterCustomTokenArgs {
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],
    #[clap(long)]
    mint: Pubkey,
    #[clap(long, value_parser = parse_token_manager_type)]
    token_manager_type: state::token_manager::Type,
    /// The token program to use for the mint. This can be either spl_token or spl_token_2022.
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,
    #[clap(long)]
    operator: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct LinkTokenArgs {
    #[clap(long, value_parser = hash_salt)]
    salt: [u8; 32],
    #[clap(long)]
    destination_chain: String,
    #[clap(long, value_parser = parse_hex_vec)]
    destination_token_address: Vec<u8>,
    #[clap(long, value_parser = parse_token_manager_type)]
    token_manager_type: state::token_manager::Type,
    #[clap(long, value_parser = parse_hex_vec)]
    link_params: Vec<u8>,
    #[clap(long)]
    gas_value: u64,
    #[clap(long)]
    gas_service: Option<Pubkey>,
    #[clap(long)]
    gas_config_account: Option<Pubkey>,
}

#[derive(Parser, Debug)]
pub(crate) struct InterchainTransferArgs {
    #[clap(long)]
    source_account: Pubkey,
    #[clap(long)]
    authority: Option<Pubkey>, // If None, uses TokenManager PDA
    #[clap(long, value_parser = hash_salt)]
    token_id: [u8; 32],
    #[clap(long)]
    destination_chain: String,
    #[clap(long)]
    destination_address: String,
    #[clap(long)]
    amount: u64,
    #[clap(long)]
    mint: Pubkey,
    /// The token program to use for the mint. This can be either spl_token or spl_token_2022.
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,
    #[clap(long)]
    gas_value: u64,
    #[clap(long)]
    gas_service: Option<Pubkey>,
    #[clap(long)]
    gas_config_account: Option<Pubkey>,
    #[clap(long)]
    timestamp: Option<i64>, // Defaults to current time if not provided
}

#[derive(Parser, Debug)]
pub(crate) struct CallContractWithInterchainTokenArgs {
    #[clap(long)]
    source_account: Pubkey,
    #[clap(long)]
    authority: Option<Pubkey>, // If None, uses TokenManager PDA
    #[clap(long, value_parser = hash_salt)]
    token_id: [u8; 32],
    #[clap(long)]
    destination_chain: String,
    #[clap(long)]
    destination_address: String,
    #[clap(long)]
    amount: u64,
    #[clap(long)]
    mint: Pubkey,
    #[clap(long, value_parser = parse_hex_vec)]
    data: Vec<u8>,
    /// The token program to use for the mint. This can be either spl_token or spl_token_2022.
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,
    #[clap(long)]
    gas_value: u64,
    #[clap(long)]
    gas_service: Option<Pubkey>,
    #[clap(long)]
    gas_config_account: Option<Pubkey>,
    #[clap(long)]
    timestamp: Option<i64>, // Defaults to current time if not provided
}

#[derive(Parser, Debug)]
pub(crate) struct CallContractWithInterchainTokenOffchainDataArgs {
    #[clap(long)]
    source_account: Pubkey,
    #[clap(long)]
    authority: Option<Pubkey>, // If None, uses TokenManager PDA
    #[clap(long, value_parser = hash_salt)]
    token_id: [u8; 32],
    #[clap(long)]
    destination_chain: String,
    #[clap(long)]
    destination_address: String,
    #[clap(long)]
    amount: u64,
    #[clap(long)]
    mint: Pubkey,

    /// Hex string with the calldata to be sent to the contract.
    #[clap(long, value_parser = parse_hex_vec)]
    data: Vec<u8>,

    /// The token program to use for the mint. This can be either spl_token or spl_token_2022.
    #[clap(long, value_parser = parse_token_program)]
    token_program: Pubkey,
    #[clap(long)]
    gas_value: u64,
    #[clap(long)]
    gas_service: Option<Pubkey>,
    #[clap(long)]
    gas_config_account: Option<Pubkey>,
    #[clap(long)]
    timestamp: Option<i64>, // Defaults to current time if not provided
}

#[derive(Parser, Debug)]
pub(crate) struct SetFlowLimitArgs {
    #[clap(long, value_parser = hash_salt)]
    token_id: [u8; 32],
    #[clap(long)]
    flow_limit: u64,
}

#[derive(Parser, Debug)]
pub(crate) struct TransferOperatorshipArgs {
    #[clap(long)]
    to: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct AcceptOperatorshipArgs {
    #[clap(long)]
    from: Pubkey,
}

pub(crate) async fn build_instruction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Instruction> {
    match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config).await,
        Commands::SetPauseStatus(set_pause_args) => {
            set_pause_status(fee_payer, set_pause_args).await
        }
        Commands::SetTrustedChain(set_trusted_chain_args) => {
            set_trusted_chain(fee_payer, set_trusted_chain_args).await
        }
        Commands::RemoveTrustedChain(args) => remove_trusted_chain(fee_payer, args).await,
        Commands::ApproveDeployRemoteInterchainToken(args) => {
            approve_deploy_remote_interchain_token(fee_payer, args, config).await
        }
        Commands::RevokeDeployRemoteInterchainToken(args) => {
            revoke_deploy_remote_interchain_token(fee_payer, args).await
        }
        Commands::RegisterCanonicalInterchainToken(args) => {
            register_canonical_interchain_token(fee_payer, args).await
        }
        Commands::DeployRemoteCanonicalInterchainToken(args) => {
            deploy_remote_canonical_interchain_token(fee_payer, args, config).await
        }
        Commands::DeployInterchainToken(args) => deploy_interchain_token(fee_payer, args).await,
        Commands::DeployRemoteInterchainToken(args) => {
            deploy_remote_interchain_token(fee_payer, args, config).await
        }
        Commands::DeployRemoteInterchainTokenWithMinter(args) => {
            deploy_remote_interchain_token_with_minter(fee_payer, args, config).await
        }
        Commands::RegisterTokenMetadata(args) => {
            register_token_metadata(fee_payer, args, config).await
        }
        Commands::RegisterCustomToken(args) => register_custom_token(fee_payer, args).await,
        Commands::LinkToken(args) => link_token(fee_payer, args, config).await,
        Commands::InterchainTransfer(args) => interchain_transfer(fee_payer, args, config).await,
        Commands::CallContractWithInterchainToken(args) => {
            call_contract_with_interchain_token(fee_payer, args, config).await
        }
        Commands::CallContractWithInterchainTokenOffchainData(args) => {
            call_contract_with_interchain_token_offchain_data(fee_payer, args, config).await
        }
        Commands::SetFlowLimit(args) => set_flow_limit(fee_payer, args).await,
        Commands::TransferOperatorship(args) => transfer_operatorship(fee_payer, args).await,
        Commands::ProposeOperatorship(args) => propose_operatorship(fee_payer, args).await,
        Commands::AcceptOperatorship(args) => accept_operatorship(fee_payer, args).await,
    }
}

// Note: The `call_contract_with_interchain_token_offchain_data` instruction builder
// returns a tuple `(Instruction, Vec<u8>)`. This CLI currently only handles the
// `Instruction`. The offchain data needs separate handling (e.g., storing or logging).

async fn init(
    fee_payer: &Pubkey,
    init_args: InitArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let its_hub_address = String::deserialize(
        &utils::chains_info(config.network_type)[AXELAR_KEY][CONTRACTS_KEY][ITS_KEY][ADDRESS_KEY],
    )?;

    Ok(axelar_solana_its::instruction::initialize(
        *fee_payer,
        axelar_solana_gateway::get_gateway_root_config_pda().0,
        init_args.operator,
        ChainNameOnAxelar::from(config.network_type).0,
        its_hub_address,
    )?)
}

async fn set_pause_status(
    fee_payer: &Pubkey,
    set_pause_args: SetPauseStatusArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::set_pause_status(
        *fee_payer,
        set_pause_args.paused,
    )?)
}

async fn set_trusted_chain(
    fee_payer: &Pubkey,
    set_trusted_chain_args: TrustedChainArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::set_trusted_chain(
        *fee_payer,
        set_trusted_chain_args.chain_name,
    )?)
}

async fn remove_trusted_chain(
    fee_payer: &Pubkey,
    remove_trusted_chain_args: TrustedChainArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::remove_trusted_chain(
        *fee_payer,
        remove_trusted_chain_args.chain_name,
    )?)
}

async fn approve_deploy_remote_interchain_token(
    fee_payer: &Pubkey,
    args: ApproveDeployRemoteInterchainTokenArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let destination_minter =
        utils::encode_its_destination(&config, &args.destination_chain, args.destination_minter)?;

    Ok(
        axelar_solana_its::instruction::approve_deploy_remote_interchain_token(
            *fee_payer,
            args.deployer,
            args.salt,
            args.destination_chain,
            destination_minter,
        )?,
    )
}

async fn revoke_deploy_remote_interchain_token(
    fee_payer: &Pubkey,
    args: RevokeDeployRemoteInterchainTokenArgs,
) -> eyre::Result<Instruction> {
    Ok(
        axelar_solana_its::instruction::revoke_deploy_remote_interchain_token(
            *fee_payer,
            args.deployer,
            args.salt,
            args.destination_chain,
        )?,
    )
}

async fn register_canonical_interchain_token(
    fee_payer: &Pubkey,
    args: RegisterCanonicalInterchainTokenArgs,
) -> eyre::Result<Instruction> {
    Ok(
        axelar_solana_its::instruction::register_canonical_interchain_token(
            *fee_payer,
            args.mint,
            args.token_program,
        )?,
    )
}

async fn deploy_remote_canonical_interchain_token(
    fee_payer: &Pubkey,
    args: DeployRemoteCanonicalInterchainTokenArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    Ok(
        axelar_solana_its::instruction::deploy_remote_canonical_interchain_token(
            *fee_payer,
            args.mint,
            args.destination_chain,
            args.gas_value,
            gas_service,
            gas_config_account,
        )?,
    )
}

async fn deploy_interchain_token(
    fee_payer: &Pubkey,
    args: DeployInterchainTokenArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::deploy_interchain_token(
        *fee_payer,
        args.salt,
        args.name,
        args.symbol,
        args.decimals,
        args.initial_supply,
        args.minter,
    )?)
}

async fn deploy_remote_interchain_token(
    fee_payer: &Pubkey,
    args: DeployRemoteInterchainTokenArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    Ok(
        axelar_solana_its::instruction::deploy_remote_interchain_token(
            *fee_payer,
            args.salt,
            args.destination_chain,
            args.gas_value,
            gas_service,
            gas_config_account,
        )?,
    )
}

async fn deploy_remote_interchain_token_with_minter(
    fee_payer: &Pubkey,
    args: DeployRemoteInterchainTokenWithMinterArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    let destination_minter =
        utils::encode_its_destination(&config, &args.destination_chain, args.destination_minter)?;
    Ok(
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
    )
}

async fn register_token_metadata(
    fee_payer: &Pubkey,
    args: RegisterTokenMetadataArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    Ok(axelar_solana_its::instruction::register_token_metadata(
        *fee_payer,
        args.mint,
        args.token_program,
        args.gas_value,
        gas_service,
        gas_config_account,
    )?)
}

async fn register_custom_token(
    fee_payer: &Pubkey,
    args: RegisterCustomTokenArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::register_custom_token(
        *fee_payer,
        args.salt,
        args.mint,
        args.token_manager_type,
        args.token_program,
        args.operator,
    )?)
}

async fn link_token(
    fee_payer: &Pubkey,
    args: LinkTokenArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    Ok(axelar_solana_its::instruction::link_token(
        *fee_payer,
        args.salt,
        args.destination_chain,
        args.destination_token_address,
        args.token_manager_type,
        args.link_params,
        args.gas_value,
        gas_service,
        gas_config_account,
    )?)
}

async fn interchain_transfer(
    fee_payer: &Pubkey,
    args: InterchainTransferArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    let timestamp: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        .try_into()?;

    let destination_address =
        utils::encode_its_destination(config, &args.destination_chain, args.destination_address)?;

    Ok(axelar_solana_its::instruction::interchain_transfer(
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
    )?)
}

async fn call_contract_with_interchain_token(
    fee_payer: &Pubkey,
    args: CallContractWithInterchainTokenArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    let timestamp: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        .try_into()?;
    let destination_address =
        utils::encode_its_destination(config, &args.destination_chain, args.destination_address)?;
    Ok(
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
    )
}

async fn call_contract_with_interchain_token_offchain_data(
    fee_payer: &Pubkey,
    args: CallContractWithInterchainTokenOffchainDataArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let gas_service = try_infer_gas_service_id(args.gas_service, config)?;
    let gas_config_account = try_infer_gas_service_config_account(args.gas_config_account, config)?;
    let timestamp: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        .try_into()?;
    let destination_address =
        utils::encode_its_destination(config, &args.destination_chain, args.destination_address)?;

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
            args.data, // This is the raw data, the function calculates the hash
            args.token_program,
            args.gas_value,
            gas_service,
            gas_config_account,
            timestamp,
        )?;

    let mut file = File::create(config.output_dir.join("offchain_data_payload.bin"))?;
    file.write(&payload)?;

    Ok(instruction)
}

async fn set_flow_limit(fee_payer: &Pubkey, args: SetFlowLimitArgs) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::set_flow_limit(
        *fee_payer,
        args.token_id,
        args.flow_limit,
    )?)
}

async fn transfer_operatorship(
    fee_payer: &Pubkey,
    args: TransferOperatorshipArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::transfer_operatorship(
        *fee_payer, args.to,
    )?)
}

async fn propose_operatorship(
    fee_payer: &Pubkey,
    args: TransferOperatorshipArgs, // Reuses args from transfer
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::propose_operatorship(
        *fee_payer, args.to,
    )?)
}

async fn accept_operatorship(
    fee_payer: &Pubkey,
    args: AcceptOperatorshipArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::accept_operatorship(
        *fee_payer, args.from,
    )?)
}
