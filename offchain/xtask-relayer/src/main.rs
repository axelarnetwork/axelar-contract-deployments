mod axelar;

use axelar::gateway_verify_messages::axelar_gateway_verify_messages;
use axelar::verifier_is_verified::axelar_dummy_verifier_is_verified;
use clap::Parser;
use common::solana::client::Client;
use common::types::{CcId, Message};
use env_logger;
use log::{info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use std::str::FromStr;
use tokio::time::{interval, Duration};

#[derive(Parser, Debug, Clone)]
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

    #[arg(long)]
    solana_tx_commitment: String,

    #[arg(long)]
    solana_tx_limit: usize,

    #[arg(long)]
    solana_rpc_addr: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    env_logger::init();
    let tick_rate = Duration::from_secs(2);
    let mut ticker = interval(tick_rate);

    let solana_basic_rpc_client = RpcClient::new(args.solana_rpc_addr);
    let solana_rpc_client: Client<'_> =
        Client::new_without_payer(&solana_basic_rpc_client, &args.solana_tx_commitment);

    loop {
        let txs = solana_rpc_client.fetch_tx_signatures_per_address(
            &gateway::id(),
            None,
            None,
            args.solana_tx_limit.clone(),
        );

        // request rate limit
        ticker.tick().await;

        for tx in txs {
            let signature = Signature::from_str(tx.signature.as_str()).unwrap();

            match solana_rpc_client.fetch_events_by_tx_signature_contract_call(signature.clone()) {
                Ok(events) => {
                    for (index, event) in events.iter().enumerate() {
                        // prep for axelar
                        let cc_id = CcId::from_chain_signature_and_index(
                            "sol".to_owned(),
                            signature.clone(),
                            index,
                        );
                        let correct_request_body =
                            Message::prepare_message_for_axelar_side(cc_id, event);

                        info!("sending: {:?}", correct_request_body.clone());

                        // push message to axelar gateway
                        axelar_gateway_verify_messages(
                            correct_request_body.clone(),
                            &args.fees,
                            &args.fees_ratio,
                            &args.axelar_payer,
                            &args.axelar_gateway_addr,
                            &args.rpc_addr,
                        );

                        // check if state has changed happy scenario
                        let _result_is_verified_verifier = axelar_dummy_verifier_is_verified(
                            correct_request_body.clone(),
                            args.axelar_verifier_addr.clone(),
                            args.rpc_addr.clone(),
                        );

                        // angry scenario
                        let _result_is_verified_verifier = axelar_dummy_verifier_is_verified(
                        Message {
                            cc_id: CcId {
                                chain: "sol".to_owned(),
                                id: "wrong-command-id-should-be-false".to_owned(),
                            },
                            source_address: event.sender.to_string(), // TODO: check if thats correct
                            destination_chain: event.destination_chain.clone(),
                            destination_address: event.destination_contract_address.clone(),
                            payload_hash:
                                "2CE2D8F68382ACFAF56AD8BF81DAFDBD558490431B701FD10F8969CD8669EB2D"
                                    .to_string(),
                        },
                        args.axelar_verifier_addr.clone(),
                        args.rpc_addr.clone(),
                    );
                    }
                }
                Err(e) => warn!(
                    "tx without correct commitment(wait for propagation): {:?}",
                    e
                ),
            };
        }
    }
}
