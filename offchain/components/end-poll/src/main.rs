use std::sync::Arc;

use cosmos_sdk_proto::cosmos::auth::v1beta1::query_client::QueryClient;
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::Coin;

use cosmrs::{
    tx::{Fee, Msg},
    AccountId,
};
use futures_util::stream::StreamExt;
use serde::Serialize;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::EventType;
use tendermint_rpc::query::Query;
use tendermint_rpc::Client;
use tendermint_rpc::HttpClient;
use tendermint_rpc::SubscriptionClient;
use tendermint_rpc::WebSocketClient;
use tokio::task::JoinSet;

use crate::account::AxelarAccount;
use crate::tx_builder::AxelarTxBuilder;

mod account;
mod tx_builder;

#[tokio::main]
async fn main() {
    let (client, driver) = WebSocketClient::new("ws://devnet.rpc.axelar.dev:26657/websocket")
        .await
        .unwrap();
    let driver_handle = tokio::spawn(async move { driver.run().await });

    let mut subs = client
        .subscribe(Query::from(EventType::Tx).and_eq(
            "wasm-messages_poll_started.source_gateway_address",
            "C3iZqLs7omGNxbug6SbeKHAAiJYArNAkn9KxudeDSdpG", // Only accept poll_started events and messages
                                                            // coming fromSolana source gateway address
        ))
        .await
        .unwrap();

    let arc_client = Arc::new(client);
    let mut join_set = JoinSet::new();
    while let Some(res) = subs.next().await {
        let ev = res.unwrap();
        let mut poll_block_expiry = u32::default();
        let mut poll_id = String::default();

        for e in ev
            .events
            .expect("expected events to be present for poll_started")
        {
            let event_attr = e.0;
            let event_data = e.1;

            if event_attr == "wasm-messages_poll_started.poll_id" {
                poll_id = event_data[0].trim_matches('"').to_owned(); //  poll_id is "\"123\""
            }

            if event_attr == "wasm-messages_poll_started.expires_at" {
                poll_block_expiry = event_data[0]
                    .parse::<u32>()
                    .expect("poll expiry block is not u64");
            }
        }

        let arc_client_clone = arc_client.clone();
        join_set.spawn(end_poll_on_expiration(
            arc_client_clone,
            poll_id, // TODO: Should pass by reference, but lifetime issues:w,
            tendermint::block::Height::from(poll_block_expiry),
        ));
    }

    while let Some(join) = join_set.join_next().await {
        join.unwrap();
    }

    // Signal to the driver to terminate.
    Arc::into_inner(arc_client).unwrap().close().unwrap();
    // Await the driver's termination to ensure proper connection closure.
    let _ = driver_handle.await.unwrap();
}

async fn end_poll_on_expiration(
    client: Arc<WebSocketClient>,
    poll_id: String,
    poll_block_expiry: tendermint::block::Height,
) {
    // Subscription functionality
    let mut subs = client
        .subscribe(Query::from(EventType::NewBlock))
        .await
        .unwrap();

    while let Some(res) = subs.next().await {
        let block_data = res.unwrap().data;
        match block_data {
            EventData::LegacyNewBlock {
                block,
                result_begin_block,
                result_end_block,
            } => {
                let block_height = block.unwrap().header.height;
                if block_height >= poll_block_expiry {
                    broadcast_endpoll_tx(&poll_id).await;
                    break;
                }
            }
            _ => (),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    EndPoll { poll_id: String },
}

async fn broadcast_endpoll_tx(poll_id: &str) {
    let fixed_seed: [u8; 32] = [1; 32];
    // TODO: This, along with other clients should be in main and passed as references
    let auth_query_client = QueryClient::connect("http://devnet.rpc.axelar.dev:9090")
        .await
        .unwrap();

    let account = AxelarAccount::new(auth_query_client, fixed_seed).await;

    let verifier_acc_id = "axelar1ck5ff02w2js52mt6q9h7gqjl9l8pvyjsd3eg63e5uxcwysmm07sqn37xaw"
        .parse::<AccountId>()
        .unwrap();

    let tx_msg = MsgExecuteContract {
        sender: account.id.clone(),
        contract: verifier_acc_id,
        funds: vec![],
        msg: serde_json::to_vec(&ExecuteMsg::EndPoll {
            poll_id: poll_id.to_string(),
        })
        .expect("endpoll msg should serialize"),
    };

    let gas = 250_000u64;
    let chain_id = "devnet-wasm"
        .parse()
        .expect("should parse to tendermint::chain::Id");
    let amount = Coin {
        amount: 1_000u128,
        denom: "uwasm".parse().expect("should parse as Denom"),
    };

    let tx = AxelarTxBuilder::new(&account)
        .set_fee(Fee::from_amount_and_gas(amount, gas))
        .set_body(vec![tx_msg.to_any().unwrap()])
        .build();

    let raw_tx = tx.sign(&account, chain_id);

    // TODO: This, along with other clients should be in main and passed as references
    // Also we should use the RPC ServiceClient, instead of HttpClient
    let client = HttpClient::new("http://devnet.rpc.axelar.dev:26657").unwrap();
    let resp = client
        .broadcast_tx_sync(raw_tx.to_bytes().unwrap())
        .await
        .unwrap();
    println!("RESPONSEEEEEEEEEE {:?}", resp);
}
