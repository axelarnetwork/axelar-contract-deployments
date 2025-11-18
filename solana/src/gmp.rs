use anchor_lang::{InstructionData, ToAccountMetas};
use clap::{Parser, Subcommand};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction as SolanaTransaction;

use crate::config::Config;
use crate::types::{SerializableSolanaTransaction, SolanaTransactionParams};
use crate::utils::fetch_latest_blockhash;

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    Send(SendArgs),
}

#[derive(Parser, Debug)]
pub(crate) struct SendArgs {
    #[clap(long)]
    destination_chain: String,

    #[clap(long)]
    destination_address: String,

    /// Hex payload
    #[clap(long, value_parser = parse_hex_payload)]
    payload: String,
}
// TODO: review
fn parse_hex_payload(s: &str) -> Result<String, String> {
    let stripped = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(stripped)
        .map(|_| stripped.to_string())
        .map_err(|e| format!("Invalid hex payload: {}", e))
}

pub(crate) fn build_transaction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<SerializableSolanaTransaction>> {
    let instruction = build_instruction(fee_payer, command, config)?;

    let blockhash = fetch_latest_blockhash(&config.url)?;

    // Message to be passed into tx (single instruction)
    let message = solana_sdk::message::Message::new_with_blockhash(
        &[instruction],
        Some(fee_payer),
        &blockhash,
    );

    // Unsigned tx
    let transaction = SolanaTransaction::new_unsigned(message);

    // Tx params
    let params = SolanaTransactionParams {
        fee_payer: fee_payer.to_string(),
        recent_blockhash: Some(blockhash.to_string()),
        nonce_account: None,
        nonce_authority: None,
        blockhash_for_message: blockhash.to_string(),
    };

    // Create serializable tx
    let serializable_tx = SerializableSolanaTransaction::new(transaction, params);

    // Return tx be signed
    Ok(vec![serializable_tx])
}

/// Build instruction for GMP commands
fn build_instruction(
    fee_payer: &Pubkey,
    command: Commands,
    _config: &Config,
) -> eyre::Result<Instruction> {
    match command {
        Commands::Send(args) => send_gmp_message(fee_payer, args),
    }
}

fn send_gmp_message(fee_payer: &Pubkey, args: SendArgs) -> eyre::Result<Instruction> {
    let payload = hex::decode(args.payload.strip_prefix("0x").unwrap_or(&args.payload))?;

    // Build gateway call instruction
    let gateway_instruction = build_gateway_call_instruction(
        fee_payer,
        &args.destination_chain,
        &args.destination_address,
        payload.clone(),
    )?;

    println!("------------------------------------------");
    println!("📨 GMP Message Details:");
    println!();
    println!("  Destination Chain: {}", args.destination_chain);
    println!("  Destination Address: {}", args.destination_address);
    println!("  Payload: {} bytes", payload.len());
    println!("------------------------------------------");

    Ok(gateway_instruction)
}

fn build_gateway_call_instruction(
    fee_payer: &Pubkey,
    destination_chain: &str,
    destination_address: &str,
    payload: Vec<u8>,
) -> eyre::Result<Instruction> {
    let gateway_config_pda = solana_axelar_gateway::GatewayConfig::find_pda().0;
    //calculate event authority pda to authorize self-CPI call
    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_gateway::id());

    let ix_data = solana_axelar_gateway::instruction::CallContract {
        // CallContract struct determines anchor discriminator
        destination_chain: destination_chain.to_string(),
        destination_contract_address: destination_address.to_string(),
        payload,
        signing_pda_bump: 0, // no signing pda needed for self-CPI call
    }
    .data(); //serialize to bytes with anchor discriminator as first 8 bytes

    // Used in <ctx> when triggering call_contract instruction
    let mut ix_accounts = solana_axelar_gateway::accounts::CallContract {
        caller: *fee_payer,
        signing_pda: None, // None for direct signer (wallet)
        gateway_root_pda: gateway_config_pda,
        event_authority: event_authority_pda,
        program: solana_axelar_gateway::id(),
    }
    .to_account_metas(None);

    // Mark the caller (first account) as signer
    ix_accounts[0].is_signer = true;

    Ok(Instruction {
        program_id: solana_axelar_gateway::id(), //send to gateway pda
        accounts: ix_accounts,
        data: ix_data,
    })
}
