use std::backtrace::Backtrace;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axelar_message_primitives::{DataPayload, EncodingScheme, U256};
use axelar_rkyv_encoding::test_fixtures::random_weight;
use axelar_rkyv_encoding::types::{HasheableMessageVec, Message, Payload};
use derive_builder::Builder;
use futures::StreamExt;
use gmp_gateway::axelar_auth_weighted::RotationDelaySecs;
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::instructions::{InitializeConfig, VerifierSetWrapper};
use gmp_gateway::state::{GatewayApprovedCommand, GatewayConfig, GatewayExecuteData};
use itertools::izip;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use solana_client::client_error::ClientErrorKind;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_client::rpc_request::RpcError;
use solana_rpc::rpc::JsonRpcConfig;
use solana_rpc_client_api::client_error::Error as RpcClientError;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use solana_test_validator::{TestValidator, TestValidatorGenesis, UpgradeableProgramInfo};
use solana_transaction_status::UiTransactionEncoding;
use test_fixtures::axelar_message::custom_message;
use test_fixtures::execute_data::prepare_execute_data;
use test_fixtures::test_setup::{self, SigningVerifierSet};
use tokio::fs::File;
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
const MESSAGE_SIZE_EXCEEDED_ERROR: i64 = -32602;
const NONCE: u64 = 55;
const OUTPUT_FILENAME: &str = "message_limits_report.csv";

const MIN_MESSAGE_SIZE: usize = 0;
const MIN_MESSAGES_PER_BATCH: usize = 1;
const MIN_SIGNERS_AMOUNT: usize = 1;
const MIN_ACCOUNTS_AMOUNT: usize = 0;

const MESSAGES_PER_BATCH_RANGE: std::ops::Range<usize> = MIN_MESSAGES_PER_BATCH..usize::MAX;
const MESSAGE_SIZE_RANGE: std::ops::Range<usize> = MIN_MESSAGE_SIZE..usize::MAX;
const SIGNERS_AMOUNT_RANGE: std::ops::Range<usize> = MIN_SIGNERS_AMOUNT..usize::MAX;
const ACCOUNTS_AMOUNT_RANGE: std::ops::Range<usize> = MIN_ACCOUNTS_AMOUNT..usize::MAX;

static BREAK_BATCH_SIZE: AtomicBool = AtomicBool::new(false);
static BREAK_MESSAGE_SIZE: AtomicBool = AtomicBool::new(false);
static BREAK_SIGNERS_SIZE: AtomicBool = AtomicBool::new(false);
static BREAK_ACCOUNTS_SIZE: AtomicBool = AtomicBool::new(false);

/// This function spins up a test validator with gateway and memo program
/// deployed and performs the flow for incoming gateway messages,
/// i.e.:
/// - 1. initialize gateway config
/// - 2. initialize execute data
/// - 3. initialize pending commands
/// - 4. approve messages
/// - 5. execute messages
/// - 6. validate message execution (this is a  CPI from the memo program to the
///   gateway)
///
/// This is done with different combinations of message size, number of messages
/// per transaction and number of signers. The combinations that end up in
/// successful iterations are recorded in a csv file saved in the directory
/// passed as argument to the function.
pub(crate) async fn generate_message_limits_report(
    output_dir: &Path,
    encoding: EncodingScheme,
) -> eyre::Result<()> {
    setup_panic_hook();
    change_log_level(LevelFilter::ERROR);
    build_contracts(Some(&[path::gateway_manifest(), path::memo_manifest()]))?;

    let file_name = get_filename(encoding);
    let file_path = output_dir.join(file_name);
    let writer = Arc::new(Mutex::new(csv_async::AsyncSerializer::from_writer(
        File::create(&file_path).await?,
    )));

    'signers: for num_signers in SIGNERS_AMOUNT_RANGE {
        let (validator, keypair) = clean_ledger_setup_validator().await;
        let initial_signers = test_setup::make_signers(
            &(0..num_signers)
                .map(|_| random_weight().into())
                .collect::<Vec<_>>(),
            NONCE,
            DOMAIN_SEPARATOR,
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

        'batch: for batch_size in MESSAGES_PER_BATCH_RANGE {
            'accounts: for num_additional_accounts in ACCOUNTS_AMOUNT_RANGE {
                let mut iterations = Vec::with_capacity(MAX_CONCURRENT_ITERATIONS);

                for message_size in MESSAGE_SIZE_RANGE {
                    let inputs = IterationInputs {
                        num_signers,
                        batch_size,
                        message_size,
                        num_additional_accounts,
                        keypair: keypair.clone(),
                        gateway_config_pda: gateway_config_pda.clone(),
                        signers: initial_signers.clone(),
                        counter_pda: counter.clone(),
                        validator_rpc_client: validator_rpc_client.clone(),
                        encoding,
                    };
                    let writer = writer.clone();

                    iterations.push(async move {
                        let iteration_output = try_iteration_with_params(inputs).await;
                        evaluate_iteration_with_side_effects(
                            iteration_output,
                            writer,
                            batch_size,
                            num_additional_accounts,
                            message_size,
                        )
                        .await;
                    });

                    if iterations.len() == MAX_CONCURRENT_ITERATIONS {
                        futures::future::join_all(iterations).await;
                        iterations = Vec::with_capacity(MAX_CONCURRENT_ITERATIONS);
                    }
                    if BREAK_MESSAGE_SIZE.swap(false, Ordering::Relaxed) {
                        break;
                    }
                    if BREAK_ACCOUNTS_SIZE.swap(false, Ordering::Relaxed) {
                        break 'accounts;
                    }
                    if BREAK_BATCH_SIZE.swap(false, Ordering::Relaxed) {
                        break 'batch;
                    }
                    if BREAK_SIGNERS_SIZE.swap(false, Ordering::Relaxed) {
                        break 'signers;
                    }
                }
            }
        }
    }

    writer.lock().await.flush().await?;
    sort_output_file(file_path).await?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Transaction missing metadata information")]
    TransactionMissingMetadata,

    #[error("File IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV file error: {0}")]
    Csv(#[from] csv_async::Error),

    #[error("Error serializing with bincode: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("RPC client error: {0}")]
    RpcClient(#[from] RpcClientError),

    #[error("Solana program error: {0}")]
    Program(#[from] solana_sdk::program_error::ProgramError),

    #[error("Gateway error: {0}")]
    Gateway(#[from] gmp_gateway::error::GatewayError),

    #[error("Error building the csv row: {0}")]
    RowBuilder(#[from] RowBuilderError),

    #[error("Error encoding data payload: {0}")]
    Payload(#[from] axelar_message_primitives::PayloadError),

    #[error("Unexpected error: {0}")]
    Unexpected(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Builder, Serialize, Deserialize)]
struct Row {
    #[serde(rename = "number_of_signers")]
    num_signers: usize,

    #[serde(rename = "number_of_messages_per_batch")]
    batch_size: usize,

    #[serde(rename = "number_of_accounts")]
    num_accounts: usize,

    #[serde(rename = "message_size(bytes)")]
    message_size: usize,

    #[serde(rename = "execute_data_size(bytes)")]
    execute_data_size: usize,

    #[serde(rename = "init_approve_messages_pda_tx_size(bytes)")]
    init_approve_messages_pda_tx_size: usize,

    #[serde(rename = "init_pending_cmd_tx_size(bytes)")]
    init_pending_cmd_tx_size: usize,

    #[serde(rename = "approve_messages_tx_size(bytes)")]
    approve_messages_tx_size: usize,

    #[serde(rename = "memo_call_tx_size(bytes)")]
    memo_call_tx_size: usize,

    total_compute_units: u64,
}

struct IterationInputs {
    num_signers: usize,
    batch_size: usize,
    message_size: usize,
    num_additional_accounts: usize,
    keypair: Arc<Keypair>,
    gateway_config_pda: Arc<Pubkey>,
    signers: Arc<SigningVerifierSet>,
    counter_pda: Arc<Pubkey>,
    validator_rpc_client: Arc<RpcClient>,
    encoding: EncodingScheme,
}

async fn initialize_programs(
    initial_signers: &SigningVerifierSet,
    keypair: Arc<Keypair>,
    validator_rpc_client: Arc<RpcClient>,
) -> Result<(Pubkey, (Pubkey, u8)), Error> {
    let (gateway_config_pda, _) = GatewayConfig::pda();
    let verifier_set = VerifierSetWrapper::new_from_verifier_set(initial_signers.verifier_set())?;
    let initialize_config = InitializeConfig {
        domain_separator: DOMAIN_SEPARATOR,
        initial_signer_sets: vec![verifier_set],
        minimum_rotation_delay: DEFAULT_MINIMUM_ROTATION_DELAY,
        operator: Pubkey::new_unique(),
        previous_signers_retention: DEFAULT_PREVIOUS_SIGNERS_RETENTION,
    };
    let instruction = gmp_gateway::instructions::initialize_config(
        keypair.pubkey(),
        initialize_config,
        gateway_config_pda,
    )?;

    submit_transaction(
        validator_rpc_client.clone(),
        keypair.clone(),
        &[instruction],
        false,
    )
    .await?;

    let counter = axelar_solana_memo_program::get_counter_pda(&gateway_config_pda);
    let instruction = axelar_solana_memo_program::instruction::initialize(
        &keypair.pubkey(),
        &gateway_config_pda,
        &counter,
    )?;

    submit_transaction(
        validator_rpc_client.clone(),
        keypair.clone(),
        &[instruction],
        false,
    )
    .await?;

    Ok((gateway_config_pda, counter))
}

async fn do_init_approve_messages_execute_data(
    inputs: &IterationInputs,
    payload: Payload,
    row_builder: &mut RowBuilder,
) -> Result<(Pubkey, u64), Error> {
    let (raw_execute_data, _) =
        prepare_execute_data(payload, inputs.signers.as_ref(), &DOMAIN_SEPARATOR);
    let raw_execute_data = raw_execute_data.to_bytes::<0>().unwrap();
    let execute_data = GatewayExecuteData::<HasheableMessageVec>::new(
        &raw_execute_data,
        inputs.gateway_config_pda.as_ref(),
        &DOMAIN_SEPARATOR,
    )?;
    let (execute_data_pda, _) = gmp_gateway::get_execute_data_pda(
        inputs.gateway_config_pda.as_ref(),
        &execute_data.hash_decoded_contents(),
    );
    let (instruction, _) = gmp_gateway::instructions::initialize_approve_messages_execute_data(
        inputs.keypair.pubkey(),
        *inputs.gateway_config_pda,
        &DOMAIN_SEPARATOR,
        &raw_execute_data,
    )?;
    let (init_approve_messages_pda_tx_size, compute_units) = submit_transaction(
        inputs.validator_rpc_client.clone(),
        inputs.keypair.clone(),
        &[instruction],
        true,
    )
    .await?;

    row_builder.init_approve_messages_pda_tx_size(init_approve_messages_pda_tx_size);
    row_builder.execute_data_size(raw_execute_data.len());

    Ok((execute_data_pda, compute_units.unwrap()))
}

async fn do_init_pending_commands(
    inputs: &IterationInputs,
    commands: Vec<OwnedCommand>,
    row_builder: &mut RowBuilder,
) -> Result<(Vec<Pubkey>, u64), Error> {
    let pubkey = inputs.keypair.pubkey();
    let (gateway_approved_command_pdas, instructions): (Vec<_>, Vec<_>) = commands
        .iter()
        .map(|command| {
            let (gateway_approved_message_pda, _bump, _seeds) =
                GatewayApprovedCommand::pda(inputs.gateway_config_pda.as_ref(), command);
            let instruction = gmp_gateway::instructions::initialize_pending_command(
                inputs.gateway_config_pda.as_ref(),
                &pubkey,
                command.clone(),
            )
            .unwrap();
            (gateway_approved_message_pda, instruction)
        })
        .unzip();
    let (init_pending_cmd_tx_size, cus) = submit_transaction(
        inputs.validator_rpc_client.clone(),
        inputs.keypair.clone(),
        &instructions,
        true,
    )
    .await?;

    row_builder.init_pending_cmd_tx_size(init_pending_cmd_tx_size);

    Ok((gateway_approved_command_pdas, cus.unwrap()))
}

async fn do_approve_messages(
    inputs: &IterationInputs,
    execute_data_pda: Pubkey,
    gateway_approved_command_pdas: &[Pubkey],
    row_builder: &mut RowBuilder,
) -> Result<u64, Error> {
    let approve_messages_instruction = gmp_gateway::instructions::approve_messages(
        execute_data_pda,
        *inputs.gateway_config_pda,
        gateway_approved_command_pdas,
        inputs.signers.verifier_set_tracker(),
    )?;

    let (approve_messages_tx_size, cus) = submit_transaction(
        inputs.validator_rpc_client.clone(),
        inputs.keypair.clone(),
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(u32::MAX),
            approve_messages_instruction,
        ],
        true,
    )
    .await?;

    row_builder.approve_messages_tx_size(approve_messages_tx_size);

    Ok(cus.unwrap())
}

async fn do_memo_program_calls(
    inputs: &IterationInputs,
    messages: Vec<Message>,
    data_payloads: &[DataPayload<'_>],
    gateway_approved_command_pdas: Vec<Pubkey>,
    row_builder: &mut RowBuilder,
) -> Result<u64, Error> {
    let mut memo_call_tx_size = 0;
    let mut total_compute_units = 0;

    for (message, data_payload, gateway_approved_command_pda) in
        izip!(messages, data_payloads, gateway_approved_command_pdas)
    {
        let instruction = axelar_executable::construct_axelar_executable_ix(
            message,
            data_payload.encode()?,
            gateway_approved_command_pda,
            *inputs.gateway_config_pda,
        )?;

        let (transaction_size, compute_units) = submit_transaction(
            inputs.validator_rpc_client.clone(),
            inputs.keypair.clone(),
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(u32::MAX),
                instruction,
            ],
            true,
        )
        .await?;

        // They should all be the same size
        memo_call_tx_size = transaction_size.max(memo_call_tx_size);
        total_compute_units += compute_units.unwrap();
    }

    row_builder.memo_call_tx_size(memo_call_tx_size);

    Ok(total_compute_units)
}

async fn try_iteration_with_params(inputs: IterationInputs) -> Result<Row, Error> {
    let mut total_compute_units = 0;
    let mut csv_row_builder = RowBuilder::default();
    csv_row_builder
        .num_signers(inputs.num_signers)
        .batch_size(inputs.batch_size)
        .num_accounts(inputs.num_additional_accounts + 1)
        .message_size(inputs.message_size);
    let payload_data: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(inputs.message_size)
        .map(char::from)
        .collect();
    let (messages, data_payloads): (Vec<Message>, Vec<DataPayload<'_>>) = (0..inputs.batch_size)
        .map(|_| {
            make_message_with_payload_data(
                payload_data.as_bytes(),
                *inputs.counter_pda,
                inputs.num_additional_accounts,
                inputs.encoding,
            )
        })
        .unzip();
    let (payload, commands) = payload_and_commands(&messages);
    let (execute_data_pda, compute_units) =
        do_init_approve_messages_execute_data(&inputs, payload, &mut csv_row_builder).await?;

    total_compute_units += compute_units;

    let (gateway_approved_command_pdas, compute_units) =
        do_init_pending_commands(&inputs, commands, &mut csv_row_builder).await?;

    total_compute_units += compute_units;

    total_compute_units += do_approve_messages(
        &inputs,
        execute_data_pda,
        &gateway_approved_command_pdas,
        &mut csv_row_builder,
    )
    .await?;

    total_compute_units += do_memo_program_calls(
        &inputs,
        messages,
        &data_payloads,
        gateway_approved_command_pdas,
        &mut csv_row_builder,
    )
    .await?;

    csv_row_builder.total_compute_units(total_compute_units);

    Ok(csv_row_builder.build()?)
}

async fn evaluate_iteration_with_side_effects(
    result: Result<Row, Error>,
    writer: Arc<Mutex<csv_async::AsyncSerializer<File>>>,
    batch_size: usize,
    num_additional_accounts: usize,
    message_size: usize,
) {
    match result {
        Err(Error::RpcClient(RpcClientError {
            request: _,
            kind:
                ClientErrorKind::RpcError(RpcError::RpcResponseError {
                    code: MESSAGE_SIZE_EXCEEDED_ERROR,
                    message,
                    ..
                }),
        })) => {
            tracing::error!("{message}");

            match (batch_size, num_additional_accounts, message_size) {
                // In case we are at the first iteration of both inner loops and we fail, we
                // reached the overall limit and should stop running.
                (MIN_MESSAGES_PER_BATCH, MIN_ACCOUNTS_AMOUNT, MIN_MESSAGE_SIZE) => {
                    BREAK_SIGNERS_SIZE.store(true, Ordering::Relaxed);
                }

                // In case we fail within the first iteration of the two innermost loops but not
                // the first iteration of the batch_size loop, it means we've stressed out the
                // possible combination of batch size, number of  accounts and message size for the
                // current number of signers, so we break the loop and move to the next number of
                // signers.
                (_, MIN_ACCOUNTS_AMOUNT, MIN_MESSAGE_SIZE) => {
                    BREAK_BATCH_SIZE.store(true, Ordering::Relaxed);
                }

                // In case we fail within the first iteration of the innermost loop but not the
                // first iteration of the other loops, it means we've stressed out the possible
                // combination of message size and number of accounts for the
                // current batch size and number of signers, so  we break the loop
                // and move to the next batch size.
                (_, _, MIN_MESSAGE_SIZE) => BREAK_ACCOUNTS_SIZE.store(true, Ordering::Relaxed),

                // In case we fail and it's not first iteration, just break the
                // innermost loop and try the next batch size.
                (_, _, _) => BREAK_MESSAGE_SIZE.store(true, Ordering::Relaxed),
            };
        }
        Ok(csv_row) => {
            writer
                .lock()
                .await
                .serialize(csv_row)
                .await
                .expect("Failed to write csv row");
        }
        Err(error) => {
            panic!("Unexpected error occurred: {error}")
        }
    };
}

async fn submit_transaction(
    rpc_client: Arc<RpcClient>,
    wallet_signer: Arc<Keypair>,
    instructions: &[Instruction],
    get_compute_units: bool,
) -> Result<(usize, Option<u64>), Error> {
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        instructions,
        Some(&wallet_signer.pubkey()),
        &[&wallet_signer],
        recent_blockhash,
    );
    let tx_size = bincode::serialize(&transaction)?.len();
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await?;

    let compute_units = if get_compute_units {
        // Loop until we get the confirmed transaction metadata.
        let transaction = loop {
            match rpc_client
                .get_transaction_with_config(
                    &signature,
                    RpcTransactionConfig {
                        encoding: Some(UiTransactionEncoding::Json),
                        commitment: Some(CommitmentConfig::confirmed()),
                        max_supported_transaction_version: None,
                    },
                )
                .await
            {
                Ok(confirmed_tx) => break confirmed_tx.transaction,
                Err(e) => tracing::error!(
                    "Error trying to fetch transaction information: {e}\nRetrying..."
                ),
            }
        };

        Some(
            transaction
                .meta
                .and_then(|meta| Option::from(meta.compute_units_consumed))
                .ok_or(Error::TransactionMissingMetadata)?,
        )
    } else {
        None
    };

    Ok((tx_size, compute_units))
}

async fn clean_ledger_setup_validator() -> (TestValidator, Keypair) {
    if PathBuf::from_str(LEDGER_PATH).unwrap().exists() {
        let _ = std::fs::remove_dir_all(LEDGER_PATH).inspect_err(|e| {
            tracing::warn!("Failed to remove ledger directory: {e}");
        });
    }
    setup_validator().await
}

async fn setup_validator() -> (TestValidator, Keypair) {
    let mut seed_validator = TestValidatorGenesis::default();
    let mut rpc_config = JsonRpcConfig::default_for_test();

    rpc_config.enable_rpc_transaction_history = true;

    seed_validator.rpc_config(rpc_config);

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

fn get_filename(encoding: EncodingScheme) -> String {
    match encoding {
        EncodingScheme::AbiEncoding => format!("abi_encoding_{OUTPUT_FILENAME}"),
        EncodingScheme::Borsh => format!("borsh_encoding_{OUTPUT_FILENAME}"),
        _ => OUTPUT_FILENAME.to_string(),
    }
}

fn make_message_with_payload_data(
    data: &[u8],
    counter_pda: Pubkey,
    num_additional_accounts: usize,
    encoding: EncodingScheme,
) -> (Message, DataPayload<'_>) {
    let accounts = (0..num_additional_accounts).fold(
        vec![AccountMeta::new(counter_pda, false)],
        |mut acc, _| {
            acc.push(AccountMeta::new(Pubkey::new_unique(), false));
            acc
        },
    );

    let payload = DataPayload::new(data, &accounts, encoding);
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

async fn sort_output_file(file: impl AsRef<Path>) -> Result<(), Error> {
    let mut reader = csv_async::AsyncDeserializer::from_reader(File::open(&file).await?);
    let results = reader
        .deserialize::<Row>()
        .collect::<Vec<Result<Row, csv_async::Error>>>()
        .await;
    let mut rows = results
        .into_iter()
        .collect::<Result<Vec<Row>, csv_async::Error>>()?;

    rows.sort_by(|a, b| {
        a.num_signers.cmp(&b.num_signers).then(
            a.batch_size
                .cmp(&b.batch_size)
                .then(a.num_accounts.cmp(&b.num_accounts))
                .then(a.message_size.cmp(&b.message_size)),
        )
    });

    let mut writer = csv_async::AsyncSerializer::from_writer(File::create(file).await?);
    for row in rows {
        writer.serialize(row).await?;
    }
    writer.flush().await?;

    Ok(())
}
