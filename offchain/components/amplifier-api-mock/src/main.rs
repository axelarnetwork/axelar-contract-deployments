#[allow(clippy::all)]
use crate::account::AxelarAccount;
use crate::tx_builder::AxelarTxBuilder;
use amplifier_api::axl_rpc::{
    self, axelar_rpc_server, GetPayloadRequest, GetPayloadResponse, SubscribeToApprovalsRequest,
    SubscribeToApprovalsResponse, VerifyRequest, VerifyResponse,
};
use axelar_wasm_std::voting::PollId;
use connection_router::state::{Address, ChainName, CrossChainId};
use cosmos_sdk_proto::cosmos::auth::v1beta1::query_client::QueryClient;
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::Coin;
use cosmrs::{
    tx::{Fee, Msg},
    AccountId,
};
// use lazy_static::lazy_static;
use multisig_prover::msg::GetProofResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Command;
use std::sync::Arc;
use std::{error::Error, net::ToSocketAddrs, pin::Pin};
use std::{io::ErrorKind, str::FromStr};
use tendermint::block::Height;
use tendermint_rpc::endpoint::broadcast;
use tendermint_rpc::event::EventData;
use tendermint_rpc::query::{EventType, Query};
use tendermint_rpc::HttpClient;
use tendermint_rpc::SubscriptionClient;
use tendermint_rpc::{Client, WebSocketClient};
use tiny_keccak::Hasher;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::UnboundedReceiverStream, Stream, StreamExt};
use tonic::{transport::Server, Status};

mod account;
mod tx_builder;

#[derive(Debug)]
pub struct AmplifierServer {}

type VerifyResponseStream = Pin<Box<dyn Stream<Item = Result<VerifyResponse, Status>> + Send>>;
type SubscribeToApprovalsResponseStream =
    Pin<Box<dyn Stream<Item = Result<SubscribeToApprovalsResponse, Status>> + Send>>;

// Destination address
solana_program::declare_id!("4hz16cS4d82cPKzvaQNzMCadyKSqzZR8bqzw8FfzYH8a");

// lazy_static! {
//     static ref payloads: Mutex<HashMap<[u8; 32], Vec<u8>>> = Mutex::new(HashMap::new());
// }

#[tonic::async_trait]
impl axelar_rpc_server::AxelarRpc for AmplifierServer {
    type SubscribeToApprovalsStream = SubscribeToApprovalsResponseStream;
    type VerifyStream = VerifyResponseStream;

    async fn verify(
        &self,
        request: tonic::Request<tonic::Streaming<VerifyRequest>>,
    ) -> std::result::Result<tonic::Response<Self::VerifyStream>, tonic::Status> {
        // 1. Call gateway verify messages
        // 2. Listen for PollStarted events for msgs coming from our gw on solana
        // 3. EndPoll on block expiry
        // 4. Check if msgs is verified and if verified call RouteMsg
        // 5. OPTIONAL - Manually check if the message passed thorugh the outgoing gateway

        let mut gmp_stream = request.into_inner();
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        // - listen for incoming gmp msgs from solana
        // - send them for axelaer verification
        tokio::spawn(async move {
            while let Some(incoming) = gmp_stream.next().await {
                match incoming {
                    Ok(msg) => {
                        let msg = msg.message.unwrap();
                        // 1. Send the msg to axelar gw for verification
                        let mut sha3 = tiny_keccak::Sha3::v256();
                        let mut payload_hash = [0u8; 32];
                        sha3.update(&msg.payload);
                        sha3.finalize(&mut payload_hash);

                        // TODO: Create payload_hash:payload map and store
                        // payloads in there for for later get_Payload calls
                        let axl_msg = connection_router::Message {
                            cc_id: CrossChainId::from_str(&msg.id).unwrap(),
                            source_address: Address::from_str(&msg.source_address).unwrap(),
                            destination_chain: ChainName::from_str(&msg.destination_chain).unwrap(),
                            destination_address: Address::from_str(&msg.destination_address)
                                .unwrap(),
                            payload_hash,
                        };

                        println!("SENDING MSG FOR VERIFICATION {}", axl_msg.cc_id);
                        let resp = broadcast_verify_tx(axl_msg.clone()).await;
                        match resp {
                            Ok(r) => {
                                println!("VERIFICATION TX SIGNATURE {}", r.hash);

                                let response_tx_arc = Arc::new(response_tx.clone());
                                tokio::spawn(async move {
                                    let tick_rate = tokio::time::Duration::from_secs(2);
                                    let mut ticker = tokio::time::interval(tick_rate);

                                    // every 2 secs query the verifier if the msg was verified,
                                    // because axelar doesn't emit the MessageVerified event
                                    loop {
                                        ticker.tick().await;
                                        let is_verified = query_is_verified(axl_msg.clone());
                                        if is_verified {
                                            println!("MSG VERIFIED {}", axl_msg.cc_id);
                                            response_tx_arc
                                                .send(Ok(VerifyResponse {
                                                    message: Some(axl_rpc::Message {
                                                        id: axl_msg.cc_id.to_string(),
                                                        source_chain: String::from("sol"),
                                                        source_address: gmp_gateway::id()
                                                            .to_string(),
                                                        destination_chain: axl_msg
                                                            .destination_chain
                                                            .to_string(),
                                                        destination_address: axl_msg
                                                            .destination_address
                                                            .to_string(),
                                                        payload: msg.payload.clone(),
                                                    }),
                                                    success: is_verified,
                                                }))
                                                .unwrap();
                                            break;
                                        }
                                    }
                                });
                            }
                            Err(err) => panic!("{}", err),
                        }
                    }
                    Err(err) => {
                        if let Some(io_err) = match_for_io_error(&err) {
                            if io_err.kind() == ErrorKind::BrokenPipe {
                                // here you can handle special case when client
                                // disconnected in unexpected way
                                eprintln!("\tclient disconnected: broken pipe");
                                break;
                            }
                        }

                        match response_tx.send(Err(err)) {
                            Ok(_) => (),
                            Err(_err) => break, // response was droped
                        }
                    }
                }
            }
            println!("verify stream ended");
        });

        // - listen for pollStarted events from axelar
        // - end the poll when needed
        // - route msg to gateway
        // - send ok response to stream
        tokio::spawn(async move {
            let (client, driver) =
                WebSocketClient::new("ws://devnet.rpc.axelar.dev:26657/websocket")
                    .await
                    .unwrap();
            tokio::spawn(async move { driver.run().await });

            let mut subs = client
                .subscribe(Query::from(EventType::Tx).and_eq(
                    "wasm-messages_poll_started.source_gateway_address",
                    "4hz16cS4d82cPKzvaQNzMCadyKSqzZR8bqzw8FfzYH8a", // Only accept poll_started events and messages
                                                                    // coming fromSolana source gateway address
                ))
                .await
                .unwrap();

            let arc_client = Arc::new(client);
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
                println!("POLL STARTED WITH ID {}", poll_id);

                // end poll
                let arc_client_clone = arc_client.clone();
                tokio::spawn(async move {
                    // Subscription functionality
                    let mut subs = arc_client_clone
                        .subscribe(Query::from(EventType::NewBlock))
                        .await
                        .unwrap();

                    while let Some(res) = subs.next().await {
                        let block_data = res.unwrap().data;
                        if let EventData::LegacyNewBlock {
                            block,
                            result_begin_block: _,
                            result_end_block: _,
                        } = block_data
                        {
                            let block_height = block.unwrap().header.height;
                            if block_height >= Height::from(poll_block_expiry) {
                                println!("POLL BLOCK EXPIRATION REACHED AT BLOCK {}", block_height);
                                broadcast_endpoll_tx(&poll_id).await;
                                break;
                            }
                        }
                    }
                });
            }
        });

        let out_stream = UnboundedReceiverStream::new(response_rx);

        Ok(tonic::Response::new(
            Box::pin(out_stream) as Self::VerifyStream
        ))
    }

    //
    // NOWHERE TO GET THE PAYLOAD FROM, SO I RETURN A STATIC ONE.
    //
    async fn get_payload(
        &self,
        _request: tonic::Request<GetPayloadRequest>,
    ) -> std::result::Result<tonic::Response<GetPayloadResponse>, tonic::Status> {
        let to =
            solana_program::pubkey::Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
                .unwrap();
        let solana_ix = solana_program::instruction::Instruction::new_with_bytes(
            to,
            b"Hello GMP!",
            vec![
                // Commented out, becaues the Memo program states:
                // If one or more accounts are provided to the signed-memo instruction,
                // all must be valid signers of the transaction for the instruction to succeed.
                // solana_program::instruction::AccountMeta::new(solana_gateway::id(), false),
                // solana_program::instruction::AccountMeta::new(to, false),
            ],
        );
        let bcs_sol_ix = bcs::to_bytes(&solana_ix).unwrap();

        Ok(tonic::Response::new(GetPayloadResponse {
            payload: bcs_sol_ix,
        }))
    }
    async fn subscribe_to_approvals(
        &self,
        _: tonic::Request<SubscribeToApprovalsRequest>,
    ) -> std::result::Result<tonic::Response<Self::SubscribeToApprovalsStream>, tonic::Status> {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async {
            let (client, driver) =
                WebSocketClient::new("ws://devnet.rpc.axelar.dev:26657/websocket")
                    .await
                    .unwrap();
            tokio::spawn(async move { driver.run().await });

            let mut subs = client
                .subscribe(
                    Query::from(EventType::Tx)
                        .and_eq("wasm-message_routed.destination_chain", "sol"),
                )
                .await
                .unwrap();

            while let Some(res) = subs.next().await {
                println!("RECEIVED MSG FOR PROVING");
                let ev = res.unwrap();

                // Construct the cc_id of the messsage being routed
                let mut cc_ids: Vec<CrossChainId> = vec![];
                for e in ev
                    .events
                    .expect("expected events to be present for poll_started")
                {
                    let event_attr = e.0;
                    let event_data = e.1;

                    if event_attr == *"wasm-message_routed.id" {
                        let cc_id_str = format!("eth:{}", event_data[0].clone());
                        cc_ids.push(
                            CrossChainId::from_str(cc_id_str.as_str())
                                .expect("cc_id to be parsed correctly"),
                        );
                    }
                }

                // Execute construct proof
                tokio::spawn(construct_proof(cc_ids));
            }
        });

        let (client, driver) = WebSocketClient::new("ws://devnet.rpc.axelar.dev:26657/websocket")
            .await
            .unwrap();
        tokio::spawn(async move { driver.run().await });

        let mut subs = client
            .subscribe(Query::from(EventType::Tx).and_eq(
                "wasm-proof_under_construction._contract_address",
                "axelar185nhd70vtu6ewwn45ra748qzhdx4cw6aue7v6fxu573vw95x0tfqvg58rm",
            ))
            .await
            .unwrap();

        // listen for ProofUnderConstruction
        tokio::spawn(async move {
            while let Some(res) = subs.next().await {
                println!("PROOF UNDER CONSTRUCTOIN");
                let ev = res.unwrap();

                let mut sess_id: u64 = 0;
                for e in ev
                    .events
                    .expect("expected events to be present for poll_started")
                {
                    let event_attr = e.0;
                    let event_data = e.1;

                    if event_attr == *"wasm-proof_under_construction.multisig_session_id" {
                        let escaped_sess_id = event_data[0].as_str();
                        // this step is needed because the id is an escaped string "\"123\""
                        sess_id = escaped_sess_id
                            .trim_matches('\\')
                            .trim_matches('"')
                            .to_string()
                            .parse()
                            .unwrap();
                    }
                }

                let tx_arc = Arc::new(tx.clone());
                tokio::spawn(async move {
                    let (client, driver) =
                        WebSocketClient::new("ws://devnet.rpc.axelar.dev:26657/websocket")
                            .await
                            .unwrap();
                    tokio::spawn(async move { driver.run().await });

                    let mut subs = client
                        .subscribe(Query::from(EventType::Tx).and_eq(
                            "wasm-signing_completed.session_id",
                            sess_id, // needs a hardcoded string ffs
                        ))
                        .await
                        .unwrap();

                    // listen for ProofUnderConstruction
                    while let Some(res) = subs.next().await {
                        println!("SIGNING COMPLETED FOR SESSION {}", sess_id);
                        let ev = res.unwrap();
                        let mut completed_at_block: u64 = 0;
                        for e in ev
                            .events
                            .expect("expected events to be present for poll_started")
                        {
                            let event_attr = e.0;
                            let event_data = e.1;

                            if event_attr == *"wasm-signing_completed.completed_at" {
                                completed_at_block = event_data[0].parse().unwrap();
                            }
                        }

                        println!("GETTING PROOF FOR SESSION {}", sess_id);
                        let proof_response = get_proof(sess_id);
                        match proof_response.status {
                            multisig_prover::msg::ProofStatus::Pending => {
                                panic!("event is signing complete, but proof is pending?")
                            }
                            multisig_prover::msg::ProofStatus::Completed { execute_data } => {
                                let kor = tx_arc.send(Ok(SubscribeToApprovalsResponse {
                                    chain: String::from("eth"), // TODO: I guess this should be the source chain?
                                    // Hardcoded for now, since we don't need it
                                    // anywhere?
                                    execute_data: execute_data.into(),
                                    block_height: completed_at_block,
                                }));
                                match kor {
                                    Ok(_) => println!("PROOF SENT"),
                                    Err(err) => println!("ERROR SENDING PROOF {:#?}", err),
                                }
                            }
                        }
                    }
                });
            }
        });

        let out_stream = UnboundedReceiverStream::new(rx);

        Ok(tonic::Response::new(
            Box::pin(out_stream) as Self::SubscribeToApprovalsStream
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = AmplifierServer {};
    Server::builder()
        .add_service(axelar_rpc_server::AxelarRpcServer::new(server))
        .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
        .await
        .unwrap();

    Ok(())
}

fn match_for_io_error(err_status: &Status) -> Option<&std::io::Error> {
    let mut err: &(dyn Error + 'static) = err_status;

    loop {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return Some(io_err);
        }

        // h2::Error do not expose std::io::Error with `source()`
        // https://github.com/hyperium/h2/pull/462
        if let Some(h2_err) = err.downcast_ref::<h2::Error>() {
            if let Some(io_err) = h2_err.get_io() {
                return Some(io_err);
            }
        }

        err = match err.source() {
            Some(err) => err,
            None => return None,
        };
    }
}

async fn broadcast_verify_tx(
    msg: connection_router::Message,
) -> Result<broadcast::tx_sync::Response, tendermint_rpc::Error> {
    let fixed_seed: [u8; 32] = [1; 32];
    // TODO: This, along with other clients should be in main and passed as references
    let auth_query_client = QueryClient::connect("http://devnet.rpc.axelar.dev:9090")
        .await
        .unwrap();

    let account = AxelarAccount::new(auth_query_client, fixed_seed).await;

    let gateway_addr = "axelar10v48ydx8rhnwv8hregan4ntf49fl83727pdwpx93q2z83splhkqqgwhxus"
        .parse::<AccountId>()
        .unwrap();

    let tx_msg = MsgExecuteContract {
        sender: account.id.clone(),
        contract: gateway_addr,
        funds: vec![],
        msg: serde_json::to_vec(&gateway::msg::ExecuteMsg::VerifyMessages(vec![msg.clone()]))
            .expect("verify msg should serialize"),
    };

    let gas = 400_000u64;
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
    client.broadcast_tx_sync(raw_tx.to_bytes().unwrap()).await
}

async fn broadcast_endpoll_tx(poll_id: &str) {
    let fixed_seed: [u8; 32] = [1; 32];
    // TODO: This, along with other clients should be in main and passed as references
    let auth_query_client = QueryClient::connect("http://devnet.rpc.axelar.dev:9090")
        .await
        .unwrap();

    let account = AxelarAccount::new(auth_query_client, fixed_seed).await;

    let verifier_acc_id = "axelar14w23u8l95aksx049526m6mvge94frwl7v0y5ut5n6clfj8pchzhsu3fgdx"
        .parse::<AccountId>()
        .unwrap();

    let tx_msg = MsgExecuteContract {
        sender: account.id.clone(),
        contract: verifier_acc_id,
        funds: vec![],
        msg: serde_json::to_vec(&voting_verifier::msg::ExecuteMsg::EndPoll {
            poll_id: PollId::from_str(poll_id).unwrap(),
        })
        .expect("endpoll msg should serialize"),
    };

    let gas = 400_000u64;
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
    println!("POLL ENDED WITH ID {}", poll_id);
    println!("POLL ENDED TX RESPONSE {:?}", resp);
}

fn query_is_verified(msg: connection_router::Message) -> bool {
    let request_json = json!({
        "is_verified": {
            "messages": [{
                "cc_id": {
                    "chain": msg.cc_id.chain,
                    "id": msg.cc_id.id,
                },
                "source_address": msg.source_address,
                "destination_chain": msg.destination_chain,
                "destination_address": msg.destination_address,
                "payload_hash": hex::encode(msg.payload_hash).to_uppercase(),
            }],
        },
    });

    let request_json =
        serde_json::to_string_pretty(&request_json).expect("Failed to convert to JSON string");

    let output = Command::new("axelard")
        .arg("query")
        .arg("wasm")
        .arg("contract-state")
        .arg("smart")
        .arg("axelar14w23u8l95aksx049526m6mvge94frwl7v0y5ut5n6clfj8pchzhsu3fgdx")
        .arg(request_json)
        .arg("--node")
        .arg("http://devnet.rpc.axelar.dev:26657")
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    #[derive(Debug, Serialize, Deserialize)]
    struct Response {
        data: Vec<(CrossChainId, bool)>,
    }

    if output.status.success() {
        // Deserialize JSON into Rust struct
        let output = String::from_utf8_lossy(&output.stdout);
        let my_data: Response = serde_json::from_str(&output).unwrap();

        my_data.data[0].1
    } else {
        panic!("error: {:#?}", String::from_utf8_lossy(&output.stderr));
    }
}

async fn construct_proof(message_ids: Vec<CrossChainId>) {
    if message_ids.is_empty() {
        panic!("empty msg_ids vector for proof construction");
    }
    let fixed_seed: [u8; 32] = [1; 32];
    let auth_query_client = QueryClient::connect("http://devnet.rpc.axelar.dev:9090")
        .await
        .unwrap();

    let account = AxelarAccount::new(auth_query_client, fixed_seed).await;

    let prover_addr = "axelar185nhd70vtu6ewwn45ra748qzhdx4cw6aue7v6fxu573vw95x0tfqvg58rm"
        .parse::<AccountId>()
        .unwrap();

    let tx_msg = MsgExecuteContract {
        sender: account.id.clone(),
        contract: prover_addr,
        funds: vec![],
        msg: serde_json::to_vec(&multisig_prover::msg::ExecuteMsg::ConstructProof {
            message_ids: message_ids.clone(),
        })
        .expect("construct proof msg should serialize"),
    };

    let gas = 500_000u64;
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
    println!("SENT CONSTRUCT PROOF FOR MSG IDS {:?}", message_ids);
    println!("CONSTRUCT PROOF TX RESPONSE {:?}", resp);
}

fn get_proof(session_id: u64) -> GetProofResponse {
    let request_json = json!({
        "get_proof": {
            "multisig_session_id": session_id.to_string(),
        },
    });

    let request_json =
        serde_json::to_string_pretty(&request_json).expect("Failed to convert to JSON string");

    let output = Command::new("axelard")
        .arg("query")
        .arg("wasm")
        .arg("contract-state")
        .arg("smart")
        .arg("axelar185nhd70vtu6ewwn45ra748qzhdx4cw6aue7v6fxu573vw95x0tfqvg58rm")
        .arg(request_json)
        .arg("--node")
        .arg("http://devnet.rpc.axelar.dev:26657")
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    #[derive(Debug, Serialize, Deserialize)]
    struct GetProofResponseData {
        pub data: GetProofResponse,
    }

    if output.status.success() {
        // Deserialize JSON into Rust struct
        let output = String::from_utf8_lossy(&output.stdout);
        let my_data: GetProofResponseData = serde_json::from_str(&output).unwrap();

        my_data.data
    } else {
        panic!("error: {:#?}", String::from_utf8_lossy(&output.stderr));
    }
}

// MOCK PROOF AND VERIFY LOGIC
// fn command_params(
//     source_chain: String,
//     source_address: String,
//     destination_address: &str,
//     payload_hash: &[u8; 32],
// ) -> Result<HexBinary, ContractError> {
//     let destination_address = solana_program::pubkey::Pubkey::from_str(destination_address)
//         .map_err(|_| ContractError::InvalidMessage {
//             reason: format!(
//                 "destination_address is not a valid Solana address: {}",
//                 destination_address
//             ),
//         })?;
//
//     Ok(bcs::to_bytes(&(
//         source_chain,
//         source_address,
//         destination_address,
//         payload_hash.to_vec(),
//     ))
//     .expect("couldn't serialize command as bcs")
//     .into())
// }
//
// fn encode_execute_data(
//     command_batch: &CommandBatch,
//     quorum: Uint256,
//     signers: Vec<(Signer, Option<Signature>)>,
// ) -> Result<HexBinary, ContractError> {
//     let signers = signers
//         .into_iter()
//         .map(|(signer, signature)| {
//             let mut signature = signature;
//             if let Some(Signature::Ecdsa(nonrecoverable)) = signature {
//                 signature = nonrecoverable
//                     .to_recoverable(
//                         command_batch.msg_digest().as_slice(),
//                         &signer.pub_key,
//                         identity,
//                     )
//                     .map(Signature::EcdsaRecoverable)
//                     .ok();
//             }
//
//             (signer, signature)
//         })
//         .collect::<Vec<_>>();
//     let input = bcs::to_bytes(&(
//         encode(&command_batch.data).to_vec(),
//         encode_proof(quorum, signers)?.to_vec(),
//     ))?;
//     Ok(input.into())
// }
//
// fn encode(data: &Data) -> HexBinary {
//     // destination chain id must be u64 for sui
//     let destination_chain_id = u256_to_u64(data.destination_chain_id);
//
//     let (commands_ids, command_types, command_params): (Vec<[u8; 32]>, Vec<String>, Vec<Vec<u8>>) =
//         data.commands
//             .iter()
//             .map(|command| {
//                 (
//                     make_command_id(&command.id),
//                     command.ty.to_string(),
//                     command.params.to_vec(),
//                 )
//             })
//             .multiunzip();
//
//     bcs::to_bytes(&(
//         destination_chain_id,
//         commands_ids,
//         command_types,
//         command_params,
//     ))
//     .expect("couldn't encode batch as bcs")
//     .into()
// }
//
// fn make_command_id(command_id: &HexBinary) -> [u8; 32] {
//     // command-ids are fixed length sequences
//     command_id
//         .to_vec()
//         .try_into()
//         .expect("couldn't convert command id to 32 byte array")
// }
//
// fn u256_to_u64(chain_id: Uint256) -> u64 {
//     chain_id
//         .to_string()
//         .parse()
//         .expect("value is larger than u64")
// }
//
// fn encode_proof(
//     quorum: Uint256,
//     signers: Vec<(Signer, Option<Signature>)>,
// ) -> Result<HexBinary, ContractError> {
//     let mut operators = make_operators_with_sigs(signers);
//     operators.sort(); // gateway requires operators to be sorted
//
//     let (addresses, weights, signatures): (Vec<_>, Vec<_>, Vec<_>) = operators
//         .iter()
//         .map(|op| {
//             (
//                 op.address.to_vec(),
//                 u256_to_u128(op.weight),
//                 op.signature.as_ref().map(|sig| sig.as_ref().to_vec()),
//             )
//         })
//         .multiunzip();
//
//     let signatures: Vec<Vec<u8>> = signatures.into_iter().flatten().collect();
//     let quorum = u256_to_u128(quorum);
//     Ok(bcs::to_bytes(&(addresses, weights, quorum, signatures))?.into())
// }
//
// fn u256_to_u128(val: Uint256) -> u128 {
//     val.to_string().parse().expect("value is larger than u128")
// }
//
// fn make_operators_with_sigs(signers_with_sigs: Vec<(Signer, Option<Signature>)>) -> Vec<Operator> {
//     signers_with_sigs
//         .into_iter()
//         .map(|(signer, sig)| Operator {
//             address: signer.pub_key.into(),
//             weight: signer.weight,
//             signature: sig,
//         })
//         .collect()
// }

// async fn verify(
//     &self,
//     request: tonic::Request<tonic::Streaming<VerifyRequest>>,
// ) -> std::result::Result<tonic::Response<Self::VerifyStream>, tonic::Status> {
//     let mut in_stream = request.into_inner();
//     let (tx, rx) = mpsc::unbounded_channel();
//
//     tokio::spawn(async move {
//         while let Some(result) = in_stream.next().await {
//             match result {
//                 Ok(v) => {
//                     let msg = v.message.unwrap();
//                     println!("---------------Incoming verification msg------------------");
//                     println!("{:#?}", msg);
//                     tx.send(Ok(VerifyResponse {
//                         message: Some(Message {
//                             id: msg.id,
//                             source_chain: msg.source_chain,
//                             source_address: msg.source_address,
//                             destination_chain: msg.destination_chain,
//                             destination_address: msg.destination_address,
//                             payload: msg.payload,
//                         }),
//                         success: true,
//                     }))
//                     .expect("working rx")
//                 }
//                 Err(err) => {
//                     if let Some(io_err) = match_for_io_error(&err) {
//                         if io_err.kind() == ErrorKind::BrokenPipe {
//                             // here you can handle special case when client
//                             // disconnected in unexpected way
//                             eprintln!("\tclient disconnected: broken pipe");
//                             break;
//                         }
//                     }
//
//                     match tx.send(Err(err)) {
//                         Ok(_) => (),
//                         Err(_err) => break, // response was droped
//                     }
//                 }
//             }
//         }
//         println!("\tstream ended");
//     });
//
//     // echo just write the same data that was received
//     let out_stream = UnboundedReceiverStream::new(rx);
//
//     Ok(tonic::Response::new(
//         Box::pin(out_stream) as Self::VerifyStream
//     ))
// }
//
// async fn subscribe_to_approvals(
//     &self,
//     _: tonic::Request<SubscribeToApprovalsRequest>,
// ) -> std::result::Result<tonic::Response<Self::SubscribeToApprovalsStream>, tonic::Status> {
//     let (tx, rx) = mpsc::unbounded_channel();
//     let destination_addr = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
//
//     let to =
//         solana_program::pubkey::Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")
//             .unwrap();
//     let solana_ix = solana_program::instruction::Instruction::new_with_bytes(
//         to,
//         b"Hello GMP!",
//         vec![
//             // Commented out, becaues the Memo program states:
//             // If one or more accounts are provided to the signed-memo instruction, all must be valid signers of the transaction for the instruction to succeed.
//             // solana_program::instruction::AccountMeta::new(solana_gateway::id(), false),
//             // solana_program::instruction::AccountMeta::new(to, false),
//         ],
//     );
//     let bcs_sol_ix = bcs::to_bytes(&solana_ix).unwrap();
//     let mut sha3 = tiny_keccak::Sha3::v256();
//     let mut bcs_sol_ix_hashed = [0u8; 32];
//     sha3.update(&bcs_sol_ix);
//     sha3.finalize(&mut bcs_sol_ix_hashed);
//
//     tokio::spawn(async move {
//         for _ in 0..0 {
//             let unique_cmd_id = hex::encode(uuid::Uuid::new_v4());
//
//             let data = Data {
//                 destination_chain_id: 1337u32.into(),
//                 commands: vec![ProverCommand {
//                     // IF THIS IS NOT UNIQUE, WE GET THE FOLLOWING ERROR ON THE GATEWAY
//                     // 1: Program 4hz16cS4d82cPKzvaQNzMCadyKSqzZR8bqzw8FfzYH8a invoke [1]
//                     // 2: Program log: Instruction: Initialize Execute Data
//                     // 3: Program log: panicked at 'assertion failed: `(left == right)`
//                     //   left: `4hz16cS4d82cPKzvaQNzMCadyKSqzZR8bqzw8FfzYH8a`,
//                     //   right: `11111111111111111111111111111111`', programs/gateway/src/processor.rs:257:5
//                     // 4: Program 4hz16cS4d82cPKzvaQNzMCadyKSqzZR8bqzw8FfzYH8a consumed 19737 of 200000 compute units
//                     // 5: Program 4hz16cS4d82cPKzvaQNzMCadyKSqzZR8bqzw8FfzYH8a failed: SBF program panicked
//                     id: HexBinary::from(unique_cmd_id.as_bytes()),
//                     ty: multisig_prover::types::CommandType::ApproveContractCall,
//                     params: command_params(
//                         "ETH".into(),
//                         "0x0".into(),
//                         destination_addr,
//                         &bcs_sol_ix_hashed,
//                     )
//                     .unwrap(),
//                 }],
//             };
//
//             let command_batch = CommandBatch {
//                 message_ids: vec![],
//                 id: BatchId::new(
//                     &vec![CrossChainId {
//                         chain: "AXELAR".to_string().try_into().unwrap(),
//                         id: "foobar".to_string().try_into().unwrap(),
//                     }],
//                     None,
//                 ),
//                 data,
//                 encoder: multisig_prover::encoding::Encoder::Bcs,
//             };
//             let quorum = 10u128;
//
//             let signer = Signer {
//                 address: Addr::unchecked("axelarvaloper1x86a8prx97ekkqej2x636utrdu23y8wupp9gk5"),
//                 weight: Uint256::from(100u128),
//                 pub_key: PublicKey::Ecdsa(
//                     HexBinary::from_hex(
//                         "037286a4f1177bea06c8e15cf6ec3df0b7747a01ac2329ca2999dfd74eff599028",
//                     )
//                     .unwrap(),
//                 ),
//             };
//             let signature = Signature::Ecdsa(
//         HexBinary::from_hex("ef5ce016a4beed7e11761e5831805e962fca3d8901696a61a6ffd3af2b646bdc3740f64643bdb164b8151d1424eb4943d03f71e71816c00726e2d68ee55600c6").unwrap().try_into().unwrap());
//             let encoded_exec_data = encode_execute_data(
//                 &command_batch,
//                 Uint256::from(quorum),
//                 vec![(signer, Some(signature))],
//             )
//             .unwrap();
//             let _ = tx.send(Ok(SubscribeToApprovalsResponse {
//                 chain: String::from("sol"),
//                 execute_data: encoded_exec_data.clone().into(),
//                 block_height: 123,
//             }));
//             // tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//         }
//     });
//
//     // echo the same data that was received
//     let out_stream = UnboundedReceiverStream::new(rx);
//
//     Ok(tonic::Response::new(
//         Box::pin(out_stream) as Self::SubscribeToApprovalsStream
//     ))
// }
