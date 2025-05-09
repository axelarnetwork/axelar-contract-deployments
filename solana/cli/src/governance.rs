use axelar_solana_governance::instructions::builder::IxBuilder;
use base64::Engine;
use clap::{Args, Subcommand};
use solana_sdk::{
    instruction::AccountMeta,
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    transaction::Transaction as SolanaTransaction
};

use crate::{
    config::Config,
    types::{ChainNameOnAxelar, SerializableSolanaTransaction, SolanaTransactionParams},
    utils::{
        fetch_latest_blockhash, parse_account_meta_string, read_json_file_from_path,
        write_json_to_file_path, ADDRESS_KEY, CHAINS_KEY, CONFIG_ACCOUNT_KEY, CONTRACTS_KEY,
        GOVERNANCE_ADDRESS_KEY, GOVERNANCE_CHAIN_KEY, GOVERNANCE_KEY, MINIMUM_PROPOSAL_ETA_DELAY_KEY,
        UPGRADE_AUTHORITY_KEY,
    },
};

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    #[clap(long_about = "Initialize the Governance program")]
    Init(InitArgs),

    #[clap(long_about = "Execute a scheduled proposal after its ETA")]
    ExecuteProposal(ExecuteProposalArgs),

    #[clap(long_about = "Execute an operator-approved proposal (bypasses ETA)")]
    ExecuteOperatorProposal(ExecuteOperatorProposalArgs),
}

#[derive(Args, Debug)]
pub(crate) struct InitArgs {
    #[clap(short, long)]
    governance_chain: String,

    #[clap(short, long)]
    governance_address: String,

    #[clap(short, long)]
    minimum_proposal_eta_delay: u32,

    #[clap(short, long)]
    operator: Pubkey,
}

// Common arguments for proposal execution
#[derive(Args, Debug, Clone)]
struct ProposalExecutionBaseArgs {
    #[clap(long, help = "Target program ID for the proposal's instruction")]
    target: Pubkey,

    #[clap(
        long,
        help = "Amount of native value (lamports) to transfer with the proposal"
    )]
    native_value: u64,

    #[clap(
        long,
        help = "Base64 encoded call data for the target program instruction"
    )]
    calldata: String,

    #[clap(
        long,
        help = "Account metas required by the target program instruction. Format: 'pubkey:is_signer:is_writable'",
        value_parser = parse_account_meta_string,
    )]
    target_accounts: Vec<AccountMeta>,

    #[clap(long, help = "Optional account to receive the native value transfer")]
    native_value_receiver: Option<Pubkey>,
}

#[derive(Args, Debug)]
pub(crate) struct ExecuteProposalArgs {
    #[clap(flatten)]
    base: ProposalExecutionBaseArgs,
}

#[derive(Args, Debug)]
pub(crate) struct ExecuteOperatorProposalArgs {
    #[clap(flatten)]
    base: ProposalExecutionBaseArgs,

    #[clap(long, help = "Operator pubkey (must be a signer of the transaction)")]
    operator: Pubkey,
}

#[derive(Args, Debug)]
pub(crate) struct TransferOperatorshipArgs {
    #[clap(long, help = "Pubkey of the new operator")]
    new_operator: Pubkey,

    #[clap(
        long,
        help = "Pubkey of the current operator (must be a signer of the transaction)"
    )]
    operator: Pubkey,
}

pub(crate) async fn build_instruction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let (config_pda, _) = axelar_solana_governance::state::GovernanceConfig::pda();

    match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config, &config_pda).await,
        Commands::ExecuteProposal(args) => execute_proposal(fee_payer, args, &config_pda).await,
        Commands::ExecuteOperatorProposal(args) => {
            execute_operator_proposal(fee_payer, args, &config_pda).await
        }
    }
}

pub(crate) async fn build_transaction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<SerializableSolanaTransaction>> {
    let instructions = build_instruction(fee_payer, command, config).await?;

    // Get blockhash
    let blockhash = fetch_latest_blockhash(&config.url)?;

    // Create a transaction for each individual instruction
    let mut serializable_transactions = Vec::with_capacity(instructions.len());

    for instruction in instructions {
        // Build message and transaction with blockhash for a single instruction
        let message = solana_sdk::message::Message::new_with_blockhash(&[instruction], Some(fee_payer), &blockhash);
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

async fn init(
    fee_payer: &Pubkey,
    init_args: InitArgs,
    config: &Config,
    config_pda: &Pubkey,
) -> eyre::Result<Vec<Instruction>> {
    let chain_hash = solana_sdk::keccak::hashv(&[init_args.governance_chain.as_bytes()]).0;
    let address_hash = solana_sdk::keccak::hashv(&[init_args.governance_address.as_bytes()]).0;

    let governance_config = axelar_solana_governance::state::GovernanceConfig::new(
        chain_hash,
        address_hash,
        init_args.minimum_proposal_eta_delay,
        init_args.operator.to_bytes(),
    );

    let mut chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    chains_info[CHAINS_KEY][ChainNameOnAxelar::from(config.network_type).0][CONTRACTS_KEY]
        [GOVERNANCE_KEY] = serde_json::json!({
        ADDRESS_KEY: axelar_solana_gateway::id().to_string(),
        CONFIG_ACCOUNT_KEY: config_pda.to_string(),
        UPGRADE_AUTHORITY_KEY: fee_payer.to_string(),
        GOVERNANCE_ADDRESS_KEY: init_args.governance_address,
        GOVERNANCE_CHAIN_KEY: init_args.governance_chain,
        MINIMUM_PROPOSAL_ETA_DELAY_KEY: init_args.minimum_proposal_eta_delay,
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    Ok(vec![IxBuilder::new()
        .initialize_config(fee_payer, config_pda, governance_config)
        .build()])
}

async fn execute_proposal(
    fee_payer: &Pubkey,
    args: ExecuteProposalArgs,
    config_pda: &Pubkey,
) -> eyre::Result<Vec<Instruction>> {
    let calldata_bytes = base64::engine::general_purpose::STANDARD.decode(args.base.calldata)?;
    let native_value_receiver_account = args
        .base
        .native_value_receiver
        .map(|pk| AccountMeta::new(pk, false));

    // Note: ETA is part of the proposal data stored on-chain, not provided here.
    // The builder calculates the proposal hash based on target, calldata, native_value.
    // The ETA value used in `with_proposal_data` is only relevant for *scheduling*,
    // not execution, but the builder requires some value. We use 0 here.
    let builder = IxBuilder::new().with_proposal_data(
        args.base.target,
        args.base.native_value,
        0,
        native_value_receiver_account,
        &args.base.target_accounts,
        calldata_bytes,
    );

    Ok(vec![builder
        .execute_proposal(fee_payer, config_pda)
        .build()])
}

async fn execute_operator_proposal(
    fee_payer: &Pubkey,
    args: ExecuteOperatorProposalArgs,
    config_pda: &Pubkey,
) -> eyre::Result<Vec<Instruction>> {
    let calldata_bytes = base64::engine::general_purpose::STANDARD.decode(args.base.calldata)?;
    let native_value_receiver_account = args
        .base
        .native_value_receiver
        .map(|pk| AccountMeta::new(pk, false));

    // ETA is irrelevant for operator execution. Use 0.
    let builder = IxBuilder::new().with_proposal_data(
        args.base.target,
        args.base.native_value,
        0,
        native_value_receiver_account,
        &args.base.target_accounts,
        calldata_bytes,
    );

    Ok(vec![builder
        .execute_operator_proposal(fee_payer, config_pda, &args.operator)
        .build()])
}
