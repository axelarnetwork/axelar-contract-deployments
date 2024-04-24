use std::error::Error;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::anyhow;
use futures::{stream, StreamExt};
use solana_sdk::signature::Signature;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tonic::transport::{Channel, Endpoint, Server, Uri};
use tonic::{Request, Response, Status, Streaming};
use tower::service_fn;

use crate::amplifier_api::amplifier_client::AmplifierClient;
use crate::amplifier_api::amplifier_server::{Amplifier, AmplifierServer};
use crate::amplifier_api::{
    self, BroadcastRequest, BroadcastResponse, GetPayloadRequest, GetPayloadResponse, Message,
    SubscribeToApprovalsRequest, SubscribeToApprovalsResponse, SubscribeToWasmEventsRequest,
    SubscribeToWasmEventsResponse, VerifyRequest, VerifyResponse,
};
use crate::state::interface::State;
use crate::transports::SolanaToAxelarMessage;

/// Generates a pseudo-random message using simple deterministic operations
/// baesed on a seed value.
fn fake_message(seed: u32) -> Message {
    let mut state = seed + 1234567;
    let scramble = |state: &mut u32| -> [u8; 4] {
        *state ^= state.wrapping_pow(4).rotate_left(1);
        state.to_le_bytes()
    };
    let payload = scramble(&mut state).to_vec();
    let mut text = || -> String { hex::encode(scramble(&mut state)) };
    Message {
        id: text(),
        source_chain: text(),
        source_address: text(),
        destination_chain: text(),
        destination_address: text(),
        payload,
    }
}

#[test]
fn test_fake_message() {
    assert_ne!(fake_message(1), fake_message(2));
    for i in 0..10_000 {
        assert_eq!(fake_message(i), fake_message(i))
    }
}

struct TestSetup {
    client: AmplifierClient<Channel>,
}

struct MockServer;
impl MockServer {
    fn verify_message(message: VerifyRequest) -> VerifyResponse {
        let Some(message) = message.message else {
            return VerifyResponse {
                message: None,
                error: Some(amplifier_api::Error {
                    error: "expected a message".into(),
                    error_code: 1,
                }),
            };
        };
        VerifyResponse {
            message: Some(message),
            error: None,
        }
    }

    async fn setup() -> anyhow::Result<TestSetup> {
        let (client, server) = tokio::io::duplex(1024);
        let mock_server = MockServer;

        tokio::spawn(async move {
            Server::builder()
                .add_service(AmplifierServer::new(mock_server))
                .serve_with_incoming(tokio_stream::once(Ok::<_, std::io::Error>(server)))
                .await
        });

        // Move client to an option so we can _move_ the inner value on the first
        // attempt to connect. All other attempts will fail.
        let mut client = Some(client);
        let channel = Endpoint::try_from("http://[::]:50051")?
            .connect_with_connector(service_fn(move |_: Uri| {
                let client = client.take();
                async move {
                    if let Some(client) = client {
                        Ok(client)
                    } else {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Client already taken",
                        ))
                    }
                }
            }))
            .await?;

        Ok(TestSetup {
            client: AmplifierClient::new(channel),
        })
    }
}

#[tonic::async_trait]
impl Amplifier for MockServer {
    type VerifyStream = ReceiverStream<Result<VerifyResponse, Status>>;

    #[tracing::instrument(skip_all)]
    async fn verify(
        &self,
        request: Request<Streaming<VerifyRequest>>,
    ) -> Result<Response<Self::VerifyStream>, Status> {
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(128);

        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(msg) => {
                        let response = Self::verify_message(msg);
                        tx.send(Ok(response)).await.expect("working receiver");
                    }
                    Err(err) => {
                        if let Err(err) = tx.send(Err(err)).await {
                            println!("MockServer failed to reply: {err}");
                            break;
                        }
                    }
                }
            }
        });
        let out_stream = ReceiverStream::new(rx);
        Ok(Response::new(out_stream))
    }

    // We don't need to implement the other methods for this test scenario.

    async fn get_payload(
        &self,
        _request: Request<GetPayloadRequest>,
    ) -> Result<Response<GetPayloadResponse>, Status> {
        unimplemented!()
    }

    type SubscribeToApprovalsStream = ReceiverStream<Result<SubscribeToApprovalsResponse, Status>>;

    async fn subscribe_to_approvals(
        &self,
        _request: Request<SubscribeToApprovalsRequest>,
    ) -> Result<Response<Self::SubscribeToApprovalsStream>, Status> {
        unimplemented!()
    }

    type SubscribeToWasmEventsStream =
        ReceiverStream<Result<SubscribeToWasmEventsResponse, Status>>;

    async fn subscribe_to_wasm_events(
        &self,
        _request: Request<SubscribeToWasmEventsRequest>,
    ) -> Result<Response<Self::SubscribeToWasmEventsStream>, Status> {
        unimplemented!()
    }

    async fn broadcast(
        &self,
        _request: Request<BroadcastRequest>,
    ) -> Result<Response<BroadcastResponse>, Status> {
        unimplemented!()
    }
}

/// Inspiration:
/// - https://github.com/hyperium/tonic/blob/eeb3268f71ae5d1107c937392389db63d8f721fb/examples/src/mock/mock.rs
/// - https://github.com/hyperium/tonic/blob/eeb3268f71ae5d1107c937392389db63d8f721fb/examples/src/streaming/server.rs
#[tokio::test]
async fn smoke_test() -> Result<(), Box<dyn Error>> {
    const NUM_MESSAGES: u32 = 1_000;

    let TestSetup { mut client } = MockServer::setup().await?;

    // Make a message stream
    let message_stream = stream::iter(0..NUM_MESSAGES).map(|i| VerifyRequest {
        message: Some(fake_message(i)),
    });

    // Connect and get our response
    let mut response_stream = client
        .verify(message_stream)
        .await?
        .into_inner()
        .take(NUM_MESSAGES as usize);

    // Compare
    let mut counter = 0;
    while let Some(response) = response_stream.next().await {
        let expected = VerifyResponse {
            message: Some(fake_message(counter)),
            error: None,
        };
        assert_eq!(response?, expected);
        counter += 1;
    }
    assert_eq!(counter, NUM_MESSAGES);

    Ok(())
}

#[derive(Default, Debug, Clone)]
struct MockState(Arc<RwLock<Signature>>);

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct TestError(#[from] anyhow::Error);

impl State<Signature> for MockState {
    type Error = TestError;

    async fn get(&self) -> Result<Option<Signature>, Self::Error> {
        let sig = self
            .0
            .read()
            .map_err(|error| anyhow!("failed to read state: {error}"))?;
        Ok(Some(*sig))
    }

    async fn set(&self, signature: Signature) -> Result<(), Self::Error> {
        let mut sig = self
            .0
            .write()
            .map_err(|error| anyhow!("failed to write state: {error}"))?;
        *sig = signature;
        Ok(())
    }
}

#[tokio::test]
async fn axelar_amplifer_basic_flow() -> anyhow::Result<()> {
    // Setup
    let cancellation_token = CancellationToken::new();
    let TestSetup { client } = MockServer::setup().await?;
    let (sender, receiver) = mpsc::channel(128);
    let state = MockState::default();
    let verifier = super::AxelarVerifier::new(client, receiver, state.clone(), cancellation_token);
    tokio::spawn(async move { verifier.run().await });

    // Prepare the message
    let signature = Signature::new_unique();
    let state_sig = || {
        state
            .0
            .read()
            .map_err(|err| anyhow!("failed to read state: {err}"))
    };
    assert_ne!(signature, *state_sig()?); // confidence check

    // Send messages for verification
    for seed in 0..10 {
        let message = SolanaToAxelarMessage {
            message: fake_message(seed),
            signature,
        };
        sender.send(message).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check if state was updated
        assert_eq!(signature, *state_sig()?);
    }

    Ok(())
}
