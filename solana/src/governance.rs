use anchor_lang::InstructionData;
use base64::Engine;
use clap::{Args, Subcommand};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction as SolanaTransaction;

use crate::config::Config;
use crate::types::{SerializableSolanaTransaction, SolanaTransactionParams};
use crate::utils::OPERATOR_KEY;
use crate::utils::{
    ADDRESS_KEY, CHAINS_KEY, CONFIG_ACCOUNT_KEY, CONTRACTS_KEY, GOVERNANCE_ADDRESS_KEY,
    GOVERNANCE_CHAIN_KEY, GOVERNANCE_KEY, MINIMUM_PROPOSAL_ETA_DELAY_KEY, UPGRADE_AUTHORITY_KEY,
    fetch_latest_blockhash, parse_account_meta_string, read_json_file_from_path,
    write_json_to_file_path,
};

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialize the InterchainGovernance program on Solana
    Init(InitArgs),

    /// Execute a scheduled proposal after its ETA has elapsed
    ExecuteProposal(ExecuteProposalArgs),

    /// Execute an operator-approved proposal (bypasses ETA)
    ExecuteOperatorProposal(ExecuteOperatorProposalArgs),
}

#[derive(Args, Debug)]
pub(crate) struct InitArgs {
    /// The name of the chain in charge of the governance
    #[clap(long)]
    governance_chain: String,

    /// The address of the governance contract on the governance chain
    #[clap(long)]
    governance_address: String,

    /// Minimum value (in seconds) for a proposal ETA
    #[clap(long)]
    minimum_proposal_eta_delay: u32,

    /// The account to receive the operator role on the Interchain Governance program on Solana
    #[clap(long)]
    operator: Pubkey,
}

// Common arguments for proposal execution
#[derive(Args, Debug, Clone)]
struct ProposalExecutionBaseArgs {
    /// Target program ID for the proposal's instruction
    target: Pubkey,

    /// The amount of native value (lamports) to transfer with the proposal
    native_value: u64,

    /// Call data for the target program instruction
    calldata: String,

    /// Account metas required by the target program instruction. Format: 'pubkey:is_signer:is_writable'
    #[clap(long, value_parser = parse_account_meta_string)]
    target_accounts: Vec<AccountMeta>,

    /// Optional receiver of native value (lamports) for the proposal
    #[clap(long)]
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

    /// Operator account, must be a signer of the transaction
    #[clap(long)]
    operator: Pubkey,
}

#[derive(Args, Debug)]
pub(crate) struct TransferOperatorshipArgs {
    /// The account to receive the operator role on the Interchain Governance program on Solana
    #[clap(long)]
    new_operator: Pubkey,

    /// The account from which the operator role is being transferred
    #[clap(long)]
    operator: Pubkey,
}

pub(crate) fn build_instruction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let (config_pda, _) = solana_axelar_governance::GovernanceConfig::find_pda();

    match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config, &config_pda),
        Commands::ExecuteProposal(args) => execute_proposal(fee_payer, args, &config_pda),
        Commands::ExecuteOperatorProposal(args) => execute_operator_proposal(args, &config_pda),
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
    config_pda: &Pubkey,
) -> eyre::Result<Vec<Instruction>> {
    let chain_hash = solana_sdk::keccak::hashv(&[init_args.governance_chain.as_bytes()]).0;
    let address_hash = solana_sdk::keccak::hashv(&[init_args.governance_address.as_bytes()]).0;

    let params = solana_axelar_governance::GovernanceConfigInit {
        chain_hash,
        address_hash,
        minimum_proposal_eta_delay: init_args.minimum_proposal_eta_delay,
        operator: init_args.operator.to_bytes(),
    };

    let mut chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    chains_info[CHAINS_KEY][&config.chain][CONTRACTS_KEY][GOVERNANCE_KEY] = serde_json::json!({
        ADDRESS_KEY: solana_axelar_governance::id().to_string(),
        CONFIG_ACCOUNT_KEY: config_pda.to_string(),
        GOVERNANCE_ADDRESS_KEY: init_args.governance_address,
        GOVERNANCE_CHAIN_KEY: init_args.governance_chain,
        MINIMUM_PROPOSAL_ETA_DELAY_KEY: init_args.minimum_proposal_eta_delay,
        OPERATOR_KEY: init_args.operator.to_string(),
        UPGRADE_AUTHORITY_KEY: fee_payer.to_string(),
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    let program_data = solana_sdk::bpf_loader_upgradeable::get_program_data_address(
        &solana_axelar_governance::id(),
    );

    let ix_data = solana_axelar_governance::instruction::InitializeConfig { params }.data();

    Ok(vec![Instruction {
        program_id: solana_axelar_governance::id(),
        accounts: vec![
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new_readonly(*fee_payer, true),
            AccountMeta::new_readonly(program_data, false),
            AccountMeta::new(*config_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: ix_data,
    }])
}

fn execute_proposal(
    _fee_payer: &Pubkey,
    args: ExecuteProposalArgs,
    config_pda: &Pubkey,
) -> eyre::Result<Vec<Instruction>> {
    let calldata_bytes = base64::engine::general_purpose::STANDARD.decode(args.base.calldata)?;

    let solana_accounts: Vec<solana_axelar_governance::SolanaAccountMetadata> = args
        .base
        .target_accounts
        .iter()
        .map(|meta| solana_axelar_governance::SolanaAccountMetadata {
            pubkey: meta.pubkey.to_bytes(),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect();

    let native_value_receiver =
        args.base
            .native_value_receiver
            .map(|pk| solana_axelar_governance::SolanaAccountMetadata {
                pubkey: pk.to_bytes(),
                is_signer: false,
                is_writable: true,
            });

    let call_data = solana_axelar_governance::ExecuteProposalCallData {
        solana_accounts,
        solana_native_value_receiver_account: native_value_receiver,
        call_data: calldata_bytes,
    };

    let mut native_value = [0u8; 32];
    #[allow(clippy::little_endian_bytes)]
    native_value[..8].copy_from_slice(&args.base.native_value.to_le_bytes());

    let execute_data = solana_axelar_governance::ExecuteProposalData {
        target_address: args.base.target.to_bytes(),
        call_data,
        native_value,
    };

    let proposal_hash = solana_axelar_governance::ExecutableProposal::hash_from_data(&execute_data);
    let (proposal_pda, _) = solana_axelar_governance::ExecutableProposal::find_pda(&proposal_hash);

    let ix_data = solana_axelar_governance::instruction::ExecuteTimelockProposal {
        execute_proposal_data: execute_data.clone(),
    }
    .data();

    let mut accounts = vec![
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(*config_pda, false),
        AccountMeta::new(proposal_pda, false),
    ];

    for meta in &execute_data.call_data.solana_accounts {
        accounts.push(AccountMeta {
            pubkey: Pubkey::new_from_array(meta.pubkey),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        });
    }

    if let Some(receiver) = &execute_data.call_data.solana_native_value_receiver_account {
        accounts.push(AccountMeta::new(
            Pubkey::new_from_array(receiver.pubkey),
            false,
        ));
    }

    accounts.push(AccountMeta::new_readonly(args.base.target, false));

    Ok(vec![Instruction {
        program_id: solana_axelar_governance::id(),
        accounts,
        data: ix_data,
    }])
}

fn execute_operator_proposal(
    args: ExecuteOperatorProposalArgs,
    config_pda: &Pubkey,
) -> eyre::Result<Vec<Instruction>> {
    let calldata_bytes = base64::engine::general_purpose::STANDARD.decode(args.base.calldata)?;

    let solana_accounts: Vec<solana_axelar_governance::SolanaAccountMetadata> = args
        .base
        .target_accounts
        .iter()
        .map(|meta| solana_axelar_governance::SolanaAccountMetadata {
            pubkey: meta.pubkey.to_bytes(),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect();

    let native_value_receiver =
        args.base
            .native_value_receiver
            .map(|pk| solana_axelar_governance::SolanaAccountMetadata {
                pubkey: pk.to_bytes(),
                is_signer: false,
                is_writable: true,
            });

    let call_data = solana_axelar_governance::ExecuteProposalCallData {
        solana_accounts,
        solana_native_value_receiver_account: native_value_receiver,
        call_data: calldata_bytes,
    };

    let mut native_value = [0u8; 32];
    #[allow(clippy::little_endian_bytes)]
    native_value[..8].copy_from_slice(&args.base.native_value.to_le_bytes());

    let execute_data = solana_axelar_governance::ExecuteProposalData {
        target_address: args.base.target.to_bytes(),
        call_data,
        native_value,
    };

    let proposal_hash = solana_axelar_governance::ExecutableProposal::hash_from_data(&execute_data);
    let (proposal_pda, _) = solana_axelar_governance::ExecutableProposal::find_pda(&proposal_hash);
    let (operator_proposal_pda, _) =
        solana_axelar_governance::OperatorProposal::find_pda(&proposal_hash);

    let ix_data = solana_axelar_governance::instruction::ExecuteOperatorProposal {
        execute_proposal_data: execute_data.clone(),
    }
    .data();

    let mut accounts = vec![
        AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        AccountMeta::new(*config_pda, false),
        AccountMeta::new(proposal_pda, false),
        AccountMeta::new_readonly(args.operator, true),
        AccountMeta::new(operator_proposal_pda, false),
    ];

    for meta in &execute_data.call_data.solana_accounts {
        accounts.push(AccountMeta {
            pubkey: Pubkey::new_from_array(meta.pubkey),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        });
    }

    if let Some(receiver) = &execute_data.call_data.solana_native_value_receiver_account {
        accounts.push(AccountMeta::new(
            Pubkey::new_from_array(receiver.pubkey),
            false,
        ));
    }

    accounts.push(AccountMeta::new_readonly(args.base.target, false));

    Ok(vec![Instruction {
        program_id: solana_axelar_governance::id(),
        accounts,
        data: ix_data,
    }])
}
