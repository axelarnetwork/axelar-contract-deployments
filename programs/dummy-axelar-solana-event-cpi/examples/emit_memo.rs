use anyhow::Result;
use dummy_axelar_solana_event_cpi::{instruction::emit_event, processor::MemoSentEvent};
use event_cpi::Discriminator;
use solana_cli_config::{Config, CONFIG_FILE};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::AccountMeta,
    signature::{Keypair, Signer},
    signer::EncodableKey,
    transaction::Transaction,
};
use solana_transaction_status::{UiCompiledInstruction, UiInstruction};
use std::path::Path;
use std::{thread, time::Duration};

/// This example demonstrates how to emit an event using the dummy_axelar_solana_event_cpi program.
/// The event emitted is MemoSentEvent, which contains the sender's public key and a memo.
/// To run this example, first deploy the dummy_axelar_solana_event_cpi program to your local solana-test-validator.
/// Then run this example with a memo message as an argument.
//
// Example:
// solana-test-validator
// cargo build-sbf
// solana program deploy ../../target/deploy/dummy_axelar_solana_event_cpi.so
// cargo run --example emit_memo "Hello devnet"
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <memo_message>", args[0]);
        std::process::exit(1);
    }

    let memo = args[1].clone();

    // Load Solana CLI config
    let config_file = CONFIG_FILE
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Unable to determine config file path"))?;

    let cli_config = if Path::new(config_file).exists() {
        Config::load(config_file)?
    } else {
        Config::default()
    };

    // Use the RPC URL from config, fallback to devnet
    let rpc_url = cli_config.json_rpc_url;
    println!("Using RPC URL: {}", rpc_url);

    let client = RpcClient::new_with_commitment(&rpc_url, CommitmentConfig::confirmed());

    // Use keypair from config
    let payer = Keypair::read_from_file(cli_config.keypair_path)
        .map_err(|_| anyhow::anyhow!("Cannot load keypair"))?;

    println!("Payer: {}", payer.pubkey());
    println!("Memo: {}", memo);

    // Create the basic instruction
    let mut instruction = emit_event(&payer.pubkey(), memo.clone())?;

    // Add required accounts for event CPI functionality
    use dummy_axelar_solana_event_cpi::ID as PROGRAM_ID;

    // Derive the event authority PDA
    let (event_authority, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[event_cpi::EVENT_AUTHORITY_SEED],
        &PROGRAM_ID,
    );

    // Add the event authority account
    instruction
        .accounts
        .push(AccountMeta::new_readonly(event_authority, false));

    // Add the program account
    instruction
        .accounts
        .push(AccountMeta::new_readonly(PROGRAM_ID, false));

    println!("Event Authority: {}", event_authority);

    // Create and send transaction
    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let signature = client.send_and_confirm_transaction(&transaction)?;
    println!("Transaction signature: {}", signature);

    println!("Attempting to retrieve transaction details...");

    let mut inner_ix: Option<UiCompiledInstruction> = None;

    for _attempt in 1..=5 {
        thread::sleep(Duration::from_secs(8));

        use solana_transaction_status::UiTransactionEncoding;
        match client.get_transaction(&signature, UiTransactionEncoding::Json) {
            Ok(result) => {
                println!("Transaction details retrieved.");
                let ix = match result
                    .transaction
                    .meta
                    .unwrap()
                    .inner_instructions
                    .unwrap()
                    .first()
                    .unwrap()
                    .instructions
                    .first()
                    .unwrap()
                {
                    UiInstruction::Parsed(_) => {
                        panic!("Expected Compiled instruction, got Parsed");
                    }
                    UiInstruction::Compiled(ix) => ix.clone(),
                };

                inner_ix = Some(ix);

                break;
            }
            Err(_e) => {
                continue;
            }
        }
    }

    let data = match inner_ix {
        Some(ref ix) => &ix.data,
        None => {
            eprintln!("Failed to retrieve transaction details after multiple attempts.");
            std::process::exit(1);
        }
    };

    let data = bs58::decode(data).into_vec().unwrap();

    assert_eq!(&data[..8], event_cpi::EVENT_IX_TAG_LE);
    assert_eq!(&data[8..16], MemoSentEvent::DISCRIMINATOR);
    let event =
        borsh::from_slice::<MemoSentEvent>(&data[16..]).expect("Failed to deserialize event data");

    println!("Decoded event: {:?}", event);
    assert_eq!(
        event,
        MemoSentEvent {
            sender: payer.pubkey(),
            memo,
        }
    );
    println!("Event matches expected values.");

    Ok(())
}
