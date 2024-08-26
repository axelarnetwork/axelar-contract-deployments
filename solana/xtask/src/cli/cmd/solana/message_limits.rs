use std::backtrace::Backtrace;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axelar_message_primitives::command::U256;
use axelar_message_primitives::{DataPayload, EncodingScheme};
use axelar_rkyv_encoding::test_fixtures::random_weight;
use axelar_rkyv_encoding::types::{Message, Payload};
use gmp_gateway::axelar_auth_weighted::RotationDelaySecs;
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::instructions::{InitializeConfig, VerifierSetWraper};
use gmp_gateway::state::{GatewayApprovedCommand, GatewayConfig, GatewayExecuteData};
use itertools::izip;
use solana_client::client_error::ClientErrorKind;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::RpcError;
use solana_rpc_client_api::client_error::Error as RpcClientError;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use solana_test_validator::{TestValidator, TestValidatorGenesis, UpgradeableProgramInfo};
use test_fixtures::axelar_message::custom_message;
use test_fixtures::execute_data::prepare_execute_data;
use test_fixtures::test_setup::{self, SigningVerifierSet};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;
use tracing::level_filters::LevelFilter;

use super::SolanaContract;
use crate::change_log_level;
use crate::cli::cmd::solana::{build_contracts, path};

const DEFAULT_MINIMUM_ROTATION_DELAY: RotationDelaySecs = 0;
const DEFAULT_PREVIOUS_SIGNERS_RETENTION: U256 = U256::from_u64(4);
const DOMAIN_SEPARATOR: [u8; 32] = [0u8; 32];
const LEDGER_PATH: &str = "/tmp/ledger";
const MAX_CONCURRENT_ITERATIONS: usize = 200;
const MESSAGES_PER_BATCH_RANGE: std::ops::Range<usize> = 1..usize::MAX;
const MESSAGE_SIZE_RANGE: std::ops::Range<usize> = 0..usize::MAX;
const NONCE: u64 = 55;
const SIGNERS_AMOUNT_RANGE: std::ops::Range<usize> = 1..usize::MAX;
const MESSAGE_SIZE_EXCEEDED_ERROR: i64 = -32602;

static BREAK_MESSAGE_SIZE: AtomicBool = AtomicBool::new(false);
static BREAK_BATCH_SIZE: AtomicBool = AtomicBool::new(false);
static BREAK_SIGNERS_SIZE: AtomicBool = AtomicBool::new(false);

/// This function spins up a test validator with gateway and memo program
/// deployed and performs the flow for incoming gateway messages,
/// i.e.:
/// - 1. initialize gateway config
/// - 2. initialize execute data
/// - 3. initialize pending commands
/// - 4. approve messages
/// - 5. execute messages
/// - 6. validate message execution
///
/// This is done with different combinations of message size, number of messages
/// per transaction and number of signers. The combinations that end up in
/// successful iterations are recorded in a csv file saved in the directory
/// passed as argument to the function.
pub(crate) async fn generate_message_limits_report(output_dir: &Path) -> eyre::Result<()> {
    setup_panic_hook();
    change_log_level(LevelFilter::ERROR);
    build_contracts(Some(&[path::gateway_manifest(), path::memo_manifest()]))?;

    let mut report_file = File::create(output_dir.join("message_limits_report.csv")).await?;
    report_file
        .write_all(b"number_of_signers,messages_per_batch,message_size(bytes)\n")
        .await?;
    let writer = Arc::new(Mutex::new(BufWriter::new(report_file)));

    'n_signers: for n_signers in SIGNERS_AMOUNT_RANGE {
        let (validator, keypair) = clean_ledger_setup_validator().await;
        let initial_signers = test_setup::make_signers(
            &(0..n_signers)
                .map(|_| random_weight().into())
                .collect::<Vec<_>>(),
            NONCE,
        );

        let keypair = Arc::new(keypair);
        let validator_rpc_client = Arc::new(validator.get_async_rpc_client());
        let (gateway_config_pda, counter) = initialize_programs(
            &initial_signers,
            keypair.clone(),
            validator_rpc_client.clone(),
        )
        .await?;
        let gateway_config_pda = Arc::new(gateway_config_pda);
        let initial_signers = Arc::new(initial_signers);
        let counter = Arc::new(counter.0);

        'n_messages: for n_messages in MESSAGES_PER_BATCH_RANGE {
            let mut iterations = Vec::with_capacity(MAX_CONCURRENT_ITERATIONS);

            for message_size in MESSAGE_SIZE_RANGE {
                let writer = writer.clone();
                let counter = counter.clone();
                let keypair = keypair.clone();
                let gateway_config_pda = gateway_config_pda.clone();
                let initial_signers = initial_signers.clone();
                let validator_rpc_client = validator_rpc_client.clone();

                iterations.push(async move {
                    match try_iteration_with_params(
                        message_size,
                        n_messages,
                        keypair.clone(),
                        gateway_config_pda.clone(),
                        initial_signers.clone(),
                        counter.clone(),
                        validator_rpc_client.clone(),
                    )
                    .await
                    {
                        Err(RpcClientError {
                            request: _,
                            kind:
                                ClientErrorKind::RpcError(RpcError::RpcResponseError {
                                    code: MESSAGE_SIZE_EXCEEDED_ERROR,
                                    ..
                                }),
                        }) => {
                            match (n_messages, message_size) {
                                // In case we are at the first iteration of both inner loops
                                // and we fail, we reached the overall limit and should stop
                                // running.
                                (1, 0) => BREAK_SIGNERS_SIZE.store(true, Ordering::Relaxed),

                                // In case we fail within the first iteration of the innermost
                                // loop but not the first iteration of the middle loop, it
                                // means we've stressed out the possible combination of batch
                                // size and message size for the current number of signers, so
                                // we break the loop and move to the next number of signers.
                                (_, 0) => BREAK_BATCH_SIZE.store(true, Ordering::Relaxed),

                                // In case we fail and it's not first iteration, just break the
                                // innermost loop and try the next batch size.
                                (_, _) => BREAK_MESSAGE_SIZE.store(true, Ordering::Relaxed),
                            };
                        }
                        Ok(()) => {
                            writer
                                .lock()
                                .await
                                .write_all(
                                    format!("{n_signers},{n_messages},{message_size}\n",)
                                        .as_bytes(),
                                )
                                .await
                                .expect("Failed to write to report file");
                        }
                        Err(err) => {
                            panic!("Unexpected error occurred: {err}")
                        }
                    }
                });

                if iterations.len() == MAX_CONCURRENT_ITERATIONS {
                    futures::future::join_all(iterations).await;
                    iterations = Vec::with_capacity(MAX_CONCURRENT_ITERATIONS);
                }

                if BREAK_MESSAGE_SIZE.swap(false, Ordering::Relaxed) {
                    break;
                }

                if BREAK_BATCH_SIZE.swap(false, Ordering::Relaxed) {
                    break 'n_messages;
                }

                if BREAK_SIGNERS_SIZE.swap(false, Ordering::Relaxed) {
                    break 'n_signers;
                }
            }
        }
    }

    writer.lock().await.shutdown().await?;

    Ok(())
}

async fn initialize_programs(
    initial_signers: &SigningVerifierSet,
    keypair: Arc<Keypair>,
    validator_rpc_client: Arc<RpcClient>,
) -> eyre::Result<(Pubkey, (Pubkey, u8))> {
    let (gateway_config_pda, _) = GatewayConfig::pda();
    let verifier_set = VerifierSetWraper::new_from_verifier_set(initial_signers.verifier_set())
        .expect("Failed to create verifier set");
    let initialize_config = InitializeConfig {
        domain_separator: DOMAIN_SEPARATOR,
        initial_signer_sets: vec![verifier_set],
        minimum_rotation_delay: DEFAULT_MINIMUM_ROTATION_DELAY,
        operator: Pubkey::new_unique(),
        previous_signers_retention: DEFAULT_PREVIOUS_SIGNERS_RETENTION,
    };

    let ix = gmp_gateway::instructions::initialize_config(
        keypair.pubkey(),
        initialize_config,
        gateway_config_pda,
    )?;

    submit_transaction(validator_rpc_client.clone(), keypair.clone(), &[ix]).await?;

    let counter = axelar_solana_memo_program::get_counter_pda(&gateway_config_pda);
    let ix = axelar_solana_memo_program::instruction::initialize(
        &keypair.pubkey(),
        &gateway_config_pda,
        &counter,
    )?;

    submit_transaction(validator_rpc_client.clone(), keypair.clone(), &[ix]).await?;

    Ok((gateway_config_pda, counter))
}

async fn try_iteration_with_params(
    message_size: usize,
    n_messages: usize,
    keypair: Arc<Keypair>,
    gateway_config_pda: Arc<Pubkey>,
    signers: Arc<SigningVerifierSet>,
    counter_pda: Arc<Pubkey>,
    validator_rpc_client: Arc<RpcClient>,
) -> Result<(), RpcClientError> {
    let payload_data = vec![0xF; message_size];
    let (messages, data_payloads): (Vec<Message>, Vec<DataPayload<'_>>) = (0..n_messages)
        .map(|_| make_message_with_payload_data(&payload_data, *counter_pda))
        .unzip();
    let (payload, commands) = payload_and_commands(&messages);
    let (raw_execute_data, _) = prepare_execute_data(payload, signers.as_ref(), &DOMAIN_SEPARATOR);
    let execute_data = GatewayExecuteData::new(
        &raw_execute_data,
        gateway_config_pda.as_ref(),
        &DOMAIN_SEPARATOR,
    )
    .expect("Failed to create execute data");
    let (execute_data_pda, _) = execute_data.pda(gateway_config_pda.as_ref());

    let (ix, _) = gmp_gateway::instructions::initialize_execute_data(
        keypair.pubkey(),
        *gateway_config_pda,
        &DOMAIN_SEPARATOR,
        &raw_execute_data,
    )
    .expect("Failed to create execute data instruction");
    submit_transaction(validator_rpc_client.clone(), keypair.clone(), &[ix]).await?;

    let pubkey = keypair.pubkey();
    let (gateway_approved_command_pdas, ixs): (Vec<_>, Vec<_>) = commands
        .iter()
        .map(|command| {
            let (gateway_approved_message_pda, _bump, _seeds) =
                GatewayApprovedCommand::pda(gateway_config_pda.as_ref(), command);
            let ix = gmp_gateway::instructions::initialize_pending_command(
                gateway_config_pda.as_ref(),
                &pubkey,
                command.clone(),
            )
            .unwrap();
            (gateway_approved_message_pda, ix)
        })
        .unzip();
    submit_transaction(validator_rpc_client.clone(), keypair.clone(), &ixs).await?;

    let approve_messages_ix = gmp_gateway::instructions::approve_messages(
        execute_data_pda,
        *gateway_config_pda,
        &gateway_approved_command_pdas,
        signers.verifier_set_tracker(),
    )
    .expect("Failed to create approve messages instruction");
    let bump_budget = ComputeBudgetInstruction::set_compute_unit_limit(u32::MAX);
    submit_transaction(
        validator_rpc_client.clone(),
        keypair.clone(),
        &[bump_budget.clone(), approve_messages_ix],
    )
    .await?;

    for (m, data_payload, gateway_approved_command_pda) in
        izip!(messages, data_payloads, gateway_approved_command_pdas)
    {
        let ix = axelar_executable::construct_axelar_executable_ix(
            m,
            data_payload.encode().unwrap(),
            gateway_approved_command_pda,
            *gateway_config_pda,
        )
        .unwrap();

        submit_transaction(
            validator_rpc_client.clone(),
            keypair.clone(),
            &[bump_budget.clone(), ix],
        )
        .await?;
    }

    Ok(())
}

async fn submit_transaction(
    rpc_client: Arc<RpcClient>,
    wallet_signer: Arc<Keypair>,
    instructions: &[Instruction],
) -> Result<Signature, RpcClientError> {
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&wallet_signer.pubkey()),
        &[&wallet_signer],
        recent_blockhash,
    );
    rpc_client.send_and_confirm_transaction(&transaction).await
}

async fn clean_ledger_setup_validator() -> (TestValidator, Keypair) {
    if PathBuf::from_str(LEDGER_PATH).unwrap().exists() {
        std::fs::remove_dir_all(LEDGER_PATH).unwrap();
    }
    setup_validator().await
}

async fn setup_validator() -> (TestValidator, Keypair) {
    let mut seed_validator = TestValidatorGenesis::default();
    let gateway_program_id = gmp_gateway::id();
    let gateway_program_path =
        super::path::contracts_artifact_dir().join(SolanaContract::GmpGateway.file());
    let memo_program_id = axelar_solana_memo_program::id();
    let memo_program_path =
        super::path::contracts_artifact_dir().join(SolanaContract::AxelarSolanaMemo.file());

    seed_validator
        .add_upgradeable_programs_with_path(&[
            UpgradeableProgramInfo {
                program_id: gateway_program_id,
                loader: solana_sdk::bpf_loader_upgradeable::id(),
                upgrade_authority: gateway_program_id,
                program_path: gateway_program_path,
            },
            UpgradeableProgramInfo {
                program_id: memo_program_id,
                loader: solana_sdk::bpf_loader_upgradeable::id(),
                upgrade_authority: memo_program_id,
                program_path: memo_program_path,
            },
        ])
        .ledger_path(LEDGER_PATH)
        .start_async()
        .await
}

fn make_message_with_payload_data(data: &[u8], counter_pda: Pubkey) -> (Message, DataPayload<'_>) {
    let payload = DataPayload::new(
        data,
        &[AccountMeta::new(counter_pda, false)],
        EncodingScheme::Borsh,
    );
    let message = custom_message(axelar_solana_memo_program::id(), &payload);

    (message, payload)
}

fn payload_and_commands(messages: &[Message]) -> (Payload, Vec<OwnedCommand>) {
    let payload = Payload::new_messages(messages.to_vec());
    let commands = messages
        .iter()
        .cloned()
        .map(OwnedCommand::ApproveMessage)
        .collect();

    (payload, commands)
}

fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = Backtrace::capture();

        tracing::error!("Panic detected: {panic_info}");
        tracing::error!("{backtrace}");
        tracing::error!("Exiting the application.");
        std::process::exit(1);
    }));
}
