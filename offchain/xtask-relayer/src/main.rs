mod axelar;
mod common;
mod solana;

use axelar::gateway_verify_messages::axelar_gateway_verify_messages;
use axelar::verifier_is_verified::axelar_dummy_verifier_is_verified;
use clap::Parser;
use common::types::{CcId, Message};
use env_logger;
use gateway::id as gateway_program_id;
use log::info;
use solana::{
    client::setup_solana_client, gateway_call_contract::gateway_contract_call_event_listener,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Account address to pay for Axelar TXs
    #[arg(short, long)]
    axelar_payer: String,

    /// Account address to pay for Axelar TXs
    #[arg(short, long)]
    solana_payer_path: String,

    /// Axelar node RPC endpoint
    #[arg(short, long)]
    rpc_addr: String,

    /// Axelar how much you will to pay for TX execution
    #[arg(short, long)]
    fees: String,

    /// Axelar fees multiplier
    #[arg(long)]
    fees_ratio: String,

    #[arg(long)]
    axelar_gateway_addr: String,

    #[arg(long)]
    axelar_verifier_addr: String,
}

fn main() {
    let args = Args::parse();
    env_logger::init();

    let client = setup_solana_client(args.solana_payer_path);

    info!("listening to solana gateway ContractCallEvent addr: {:?} pushing to axelar gateway addr: {:?}", gateway_program_id(), args.axelar_gateway_addr.clone());

    // solana - observe / catch
    let event =
        gateway_contract_call_event_listener(&client.unwrap(), gateway_program_id()).unwrap(); // TODO: error handling

    info!(
        "event - dst_chain: {:?} | dst_contract {:?} | payload {:?} | payload_hash {:?} | sender: {:?}",
        event.destination_chain,
        event.destination_contract_address,
        event.payload,
        event.payload_hash,
        event.sender
    );

    // axelar - verify message
    // TODO: might be potentially bug here
    let payload_hash_hex: String = event
        .payload_hash
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect();
    let _result_axelar_gateway = axelar_gateway_verify_messages(
        Message {
            cc_id: CcId {
                chain: "solana".to_owned(),
                id: payload_hash_hex.clone(), // TODO: it cant be like that / should be unique
            },
            source_address: event.sender.to_string(), // TODO: check if thats correct
            destination_chain: event.destination_chain.clone(),
            destination_address: event.destination_contract_address.clone(),
            payload_hash: payload_hash_hex.clone(),
        },
        &args.fees,
        &args.fees_ratio,
        args.axelar_payer.as_str(),
        args.axelar_gateway_addr.clone(),
        args.rpc_addr.clone(),
    );

    // check if state has changed - happy scenario
    let _result_is_verified_verifier = axelar_dummy_verifier_is_verified(
        Message {
            cc_id: CcId {
                chain: "solana".to_owned(),
                id: payload_hash_hex.clone(),
            },
            source_address: event.sender.to_string(), // TODO: check if thats correct
            destination_chain: event.destination_chain.clone(),
            destination_address: event.destination_contract_address.clone(),
            payload_hash: payload_hash_hex.clone(),
        },
        args.axelar_verifier_addr.clone(),
        args.rpc_addr.clone(),
    );

    // check if state has changed - angry scenario
    let _result_is_verified_verifier = axelar_dummy_verifier_is_verified(
        Message {
            cc_id: CcId {
                chain: "solana".to_owned(),
                id: "wrong-command-id-should-be-false".to_owned(),
            },
            source_address: event.sender.to_string(), // TODO: check if thats correct
            destination_chain: event.destination_chain.clone(),
            destination_address: event.destination_contract_address.clone(),
            payload_hash: payload_hash_hex.clone(),
        },
        args.axelar_verifier_addr.clone(),
        args.rpc_addr.clone(),
    );
}
