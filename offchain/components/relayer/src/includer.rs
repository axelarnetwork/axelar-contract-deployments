mod error;

use std::convert::Infallible as Never;
use std::sync::Arc;

use axelar_executable::axelar_message_primitives::command::DecodedCommand;
use futures::future::try_join_all;
use futures::stream::{FuturesUnordered, StreamExt};
use gmp_gateway::instructions;
use gmp_gateway::state::{GatewayApprovedCommand, GatewayExecuteData};
use solana_client::client_error::ClientErrorKind;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::RpcClientConfig;
use solana_client::rpc_request::RpcError;
use solana_sdk::account::Account;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};
use url::Url;

use self::error::IncluderError;
use crate::amplifier_api::SubscribeToApprovalsResponse;
use crate::retrying_http_sender::RetryingHttpSender;
use crate::state::State;

/// Solana Includer
///
/// Submits incoming Axelar Messages to the Solana Gateway.
#[allow(dead_code)]
pub struct SolanaIncluder {
    rpc: Url,
    keypair: Arc<Keypair>,
    gateway_address: Pubkey,
    gateway_config_address: Pubkey,
    receiver: Receiver<SubscribeToApprovalsResponse>,
    state: State,
    cancellation_token: CancellationToken,
}

impl SolanaIncluder {
    pub fn new(
        rpc: Url,
        keypair: Arc<Keypair>,
        gateway_address: Pubkey,
        gateway_config_address: Pubkey,
        receiver: Receiver<SubscribeToApprovalsResponse>,
        state: State,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            rpc,
            keypair,
            receiver,
            gateway_address,
            gateway_config_address,
            state,
            cancellation_token,
        }
    }

    /// Creates a new `Worker` based on this `SolanaIncluder`
    fn worker(&self) -> Worker {
        let client = {
            let sender = RetryingHttpSender::new(self.rpc.to_string());
            let config = RpcClientConfig::with_commitment(CommitmentConfig::confirmed());
            let client = RpcClient::new_sender(sender, config);
            Arc::new(client)
        };

        Worker {
            client,
            keypair: self.keypair.clone(),
            gateway_address: self.gateway_address,
            gateway_config_address: self.gateway_config_address,
            state: self.state.clone(),
        }
    }

    /// Tries to run [`SolanaIncluder`] forever.
    #[tracing::instrument(name = "solana-includer", skip(self))]
    pub async fn run(self) {
        info!("task started");

        // Includer task should run forever, but it returned with some error.
        // Unwrap: Includer never returns without errors.
        let includer_error = self.work().await.unwrap_err();
        error!(%includer_error, "terminating");
    }

    /// Listens for incoming messages from the Axelar Approver and process them
    /// concurrently until an error is found or the shutdown signal is received.
    async fn work(mut self) -> Result<Never, IncluderError> {
        let worker = self.worker();
        let receiver = &mut self.receiver;
        let mut futures = FuturesUnordered::new();

        // Listen for new approvals and process them concurrently until an error occurs
        // or a shutdown signal is received.
        loop {
            tokio::select! {
                // Listen for the cancellation signal.
                _ = self.cancellation_token.cancelled() => {
                    trace!("received the shutdown signal");
                    return Err(IncluderError::Cancelled)
                }

                // Listen for new messages and start working on them.
                message = receiver.recv() => {
                    trace!("received a new message");
                    let approval = message.ok_or(IncluderError::ChannelClosed)?;
                    let worker_clone = worker.clone();
                    futures.push(async move {worker_clone.include(approval).await});
                }

                // Advance the `FuturesUnordered` internal state and return
                // fatal errors, if any.
                Some(Err(error)) = futures.next() => {
                    if error.is_fatal() {
                        return Err(error);
                    };
                    error!(%error, "non-fatal error");
                }
            };
        }
    }
}

/// Possible command account statuses for operatorship transfer or call contract
/// variants.
enum CommandAccountState {
    /// The command account does not exist.
    Uninitialized,
    /// Command account exists and is in the desired state, so initialization
    /// can be skipped.
    Initialized,
    /// Command account has already been executed, so initializing it would lead
    /// to an error.
    Executed,
}

/// Worker struct for Solana inclusion process.
///
/// Performs all steps required to include an Axelar message into Solana.
///
/// Necessary because `SolanaIncluder` cannot be referenced while its input
/// channel is being listened to.
#[derive(Clone)]
struct Worker {
    client: Arc<RpcClient>,
    keypair: Arc<Keypair>,
    gateway_address: Pubkey,
    gateway_config_address: Pubkey,
    state: State,
}

impl Worker {
    /// Tries to include the Axelar approved message into the Solana Gateway.
    #[tracing::instrument(skip_all)]
    async fn include(&self, approval: SubscribeToApprovalsResponse) -> Result<(), IncluderError> {
        let axelar_block_height = approval.block_height;

        // Try to reuse the block hash for upcoming transactions if  possible.
        // If hashes get too old, we could then move this call into the individual
        // tasks/futures.
        let recent_blockhash = self
            .client
            .get_latest_blockhash()
            .await
            .map_err(IncluderError::LatestBlockHash)?;

        // Initialize the execute_data account
        //
        // Optimization: We could send the initialize_execute_data transaction
        // in parallel with the initialize_pending_command transactions.
        let (execute_data, execute_data_account) = self
            .submit_initialize_execute_data_transaction(approval, recent_blockhash)
            .await?;

        // Initialize command accounts.
        let command_accounts = self
            .initialize_pending_commands(&execute_data, recent_blockhash)
            .await?;

        // Either approve the message batch or rotate the signers.
        self.submit_transaction(
            execute_data,
            execute_data_account,
            &command_accounts,
            recent_blockhash,
        )
        .await?;

        // Persist the current block number
        self.state
            .update_axelar_block_height(axelar_block_height.try_into()?)
            .await?;

        Ok(())
    }

    /// Sends a transaction with either the `ApproveMessages` or `RotateSigners`
    /// instruction to the Gateway.
    #[tracing::instrument(skip_all, err)]
    async fn submit_transaction(
        &self,
        execute_data: GatewayExecuteData,
        execute_data_account: Pubkey,
        command_accounts: &[Pubkey],
        recent_blockhash: Hash,
    ) -> Result<Signature, IncluderError> {
        // Peek into the first command type to determine the appropriate instruction.
        // The Gateway should fail if mixed command types are used in the same
        // instruction, but we are not responsible for filtering those here.
        let instruction = match execute_data.command_batch.commands[..] {
            [] => return Err(IncluderError::EmptyCommandsList),
            [DecodedCommand::RotateSigners(_), ..] => {
                let &[command_account] = command_accounts else {
                    return Err(
                        IncluderError::MissingOrMultipleRotateSignersCommandAccounts {
                            length: command_accounts.len(),
                        },
                    );
                };
                instructions::rotate_signers(
                    self.gateway_address,
                    execute_data_account,
                    self.gateway_config_address,
                    command_account,
                )
                .map_err(IncluderError::RotateSignersInstruction)?
            }
            [DecodedCommand::ApproveMessages(_), ..] => instructions::approve_messages(
                self.gateway_address,
                execute_data_account,
                self.gateway_config_address,
                command_accounts,
            )
            .map_err(IncluderError::ApproveMessagesInstruction)?,
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_blockhash,
        );
        let signature = self
            .client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(IncluderError::ExecuteTransaction)?;
        info!(%signature, %execute_data_account, "called `execute`");
        Ok(signature)
    }

    /// Sends a transaction with the `initialize_execute_data` instruction to
    /// the Gateway.
    #[tracing::instrument(skip_all, fields(recent_blockhash))]
    async fn submit_initialize_execute_data_transaction(
        &self,
        approval: SubscribeToApprovalsResponse,
        recent_blockhash: Hash,
    ) -> Result<(GatewayExecuteData, Pubkey), IncluderError> {
        let (instruction, execute_data) = instructions::initialize_execute_data(
            self.keypair.pubkey(),
            self.gateway_config_address,
            approval.execute_data,
        )
        .map_err(IncluderError::InitializeExecuteDataInstruction)?;
        let (execute_data_pda, ..) = execute_data.pda(&self.gateway_config_address);

        // Exit early if execute account is already initialized.
        if self.fetch_solana_account(execute_data_pda).await?.is_some() {
            info!("execute_data account is already initialized");
            return Ok((execute_data, execute_data_pda));
        };
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_blockhash,
        );
        let signature = self
            .client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|client_error| {
                IncluderError::InitializeExecuteDataTransaction(client_error)
            })?;

        let (account, ..) = execute_data.pda(&self.gateway_config_address);
        debug!(%signature, %account, "initialized execute_data account");
        Ok((execute_data, account))
    }

    /// Fetch a given Solana account, if it exists.
    #[tracing::instrument(skip(self), err)]
    async fn fetch_solana_account(
        &self,
        account: Pubkey,
    ) -> Result<Option<Account>, IncluderError> {
        self.client
            .get_account(&account)
            .await
            .map(Some)
            .or_else(|error| match error.kind() {
                // Account doesn't exist
                ClientErrorKind::RpcError(RpcError::ForUser(_)) => Ok(None),
                _other_error => {
                    Err(IncluderError::AccountPreInitializationCheck { error, account })
                }
            })
    }

    /// Sends a transaction with the `initialize_pending_command` instruction to
    /// the Gateway. Returns the address of the command account.
    #[tracing::instrument(skip_all, err, fields(command_id=hex::encode(command.command_id())))]
    async fn initialize_pending_command(
        &self,
        command: DecodedCommand,
        recent_blockhash: Hash,
    ) -> Result<Pubkey, IncluderError> {
        // Check if the command account has been initialized or executed.
        let (command_address, ..) =
            GatewayApprovedCommand::pda(&self.gateway_config_address, &command);
        let CommandAccountState::Uninitialized =
            self.check_approved_command_status(command_address).await?
        else {
            return Ok(command_address);
        };
        let instruction = instructions::initialize_pending_command(
            &self.gateway_config_address,
            &self.keypair.pubkey(),
            command,
        )
        .map_err(IncluderError::InitializePendingCommandInstruction)?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_blockhash,
        );
        let signature = self
            .client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|client_error| {
                IncluderError::InitializePendingCommandTransaction(client_error)
            })?;
        debug!(%signature, "initialized pending command account");

        Ok(command_address)
    }

    /// Checks whether the approved command PDA exists or has been initialized.
    #[tracing::instrument(skip(self), err)]
    async fn check_approved_command_status(
        &self,
        command_pda: Pubkey,
    ) -> Result<CommandAccountState, IncluderError> {
        let Some(account) = self.fetch_solana_account(command_pda).await? else {
            trace!("command account is uninitialized");
            return Ok(CommandAccountState::Uninitialized);
        };
        let approved_command: GatewayApprovedCommand = borsh::from_slice(&account.data)
            .map_err(IncluderError::ApprovedCommandDeserialization)?;
        if approved_command.is_command_pending() {
            info!("command account has already been initialized");
            Ok(CommandAccountState::Initialized)
        } else {
            info!("command account has already been executed");
            Ok(CommandAccountState::Executed)
        }
    }

    /// Initialize all pending commands for the given ´execute_data`.
    #[tracing::instrument(skip_all, err, fields(command_batch = hex::encode(execute_data.command_batch_hash)))]
    async fn initialize_pending_commands(
        &self,
        execute_data: &GatewayExecuteData,
        recent_blockhash: Hash,
    ) -> Result<Vec<Pubkey>, IncluderError> {
        let command_addresses =
            try_join_all(
                execute_data.command_batch.commands.iter().map(|command| {
                    self.initialize_pending_command(command.clone(), recent_blockhash)
                }),
            )
            .await?;
        debug!("initialized all pending command accounts");
        Ok(command_addresses)
    }
}
