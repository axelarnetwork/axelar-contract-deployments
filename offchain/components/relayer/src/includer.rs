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
use solana_client::rpc_request::RpcError;
use solana_sdk::account::Account;
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
        let receiver = &mut self.receiver;
        let solana_client = RpcClient::new(self.rpc.to_string());

        // Creates a worker future for this context.
        let worker_future = |approval: SubscribeToApprovalsResponse| {
            Self::include(
                &solana_client,
                self.gateway_address,
                self.gateway_config_address,
                approval,
                self.keypair.clone(),
                self.state.clone(),
            )
        };

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
                    futures.push(worker_future(approval));
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

    /// Tries to include the Axelar approved message into the Solana Gateway.
    #[tracing::instrument(skip_all)]
    async fn include(
        rpc: &RpcClient,
        gateway_address: Pubkey,
        gateway_config_pda: Pubkey,
        approval: SubscribeToApprovalsResponse,
        payer: Arc<Keypair>,
        state: State,
    ) -> Result<(), IncluderError> {
        let axelar_block_height = approval.block_height;

        // Try to reuse the block hash for upcoming transactions if  possible.
        // If hashes get too old, we could then move this call into the individual
        // tasks/futures.
        let recent_blockhash = rpc
            .get_latest_blockhash()
            .await
            .map_err(IncluderError::LatestBlockHash)?;

        // Initialize the execute_data account
        //
        // Optimization: We could send the initialize_execute_data transaction
        // in parallel with the initialize_pending_command transactions.
        let (execute_data, execute_data_account) =
            Self::submit_initialize_execute_data_transaction(
                rpc,
                gateway_config_pda,
                approval,
                payer.clone(),
                recent_blockhash,
            )
            .await?;

        // Initialize command accounts.
        let command_accounts = Self::initialize_pending_commands(
            rpc,
            gateway_config_pda,
            execute_data,
            payer.clone(),
            recent_blockhash,
        )
        .await?;

        // Call excute
        Self::submit_execute_transaction(
            rpc,
            gateway_address,
            gateway_config_pda,
            execute_data_account,
            &command_accounts,
            payer,
            recent_blockhash,
        )
        .await?;

        // Persist the current block number
        state
            .update_axelar_block_height(axelar_block_height.try_into()?)
            .await?;

        Ok(())
    }

    /// Sends a transaction with the `initialize_execute_data` instruction to
    /// the Gateway.
    #[tracing::instrument(skip_all, fields(recent_blockhash))]
    async fn submit_initialize_execute_data_transaction(
        rpc: &RpcClient,
        gateway_config_pda: Pubkey,
        approval: SubscribeToApprovalsResponse,
        payer: Arc<Keypair>,
        recent_blockhash: Hash,
    ) -> Result<(GatewayExecuteData, Pubkey), IncluderError> {
        let (instruction, execute_data) = instructions::initialize_execute_data(
            payer.pubkey(),
            gateway_config_pda,
            approval.execute_data,
        )
        .map_err(IncluderError::InitializeExecuteDataInstruction)?;
        let (execute_data_pda, ..) = execute_data.pda(&gateway_config_pda);

        // Exit early if execute account is already initialized.
        if Self::fetch_solana_account(rpc, execute_data_pda)
            .await?
            .is_some()
        {
            info!("execute_data account is already initialized");
            return Ok((execute_data, execute_data_pda));
        };
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        let signature = rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|client_error| {
                IncluderError::InitializeExecuteDataTransaction(client_error)
            })?;

        let (account, ..) = execute_data.pda(&gateway_config_pda);
        debug!(%signature, %account, "initialized execute_data account");
        Ok((execute_data, account))
    }

    /// Sends a transaction with the `initialize_pending_command` instruction to
    /// the Gateway. Returns the address of the command account.
    #[tracing::instrument(skip_all, err, fields(command_id=hex::encode(command.command_id())))]
    async fn initialize_pending_command(
        rpc: &RpcClient,
        gateway_config_pda: Pubkey,
        payer: Arc<Keypair>,
        command: DecodedCommand,
        recent_blockhash: Hash,
    ) -> Result<Pubkey, IncluderError> {
        // Check if the command account has been initialized or executed.
        let (command_address, ..) = GatewayApprovedCommand::pda(&gateway_config_pda, &command);
        let CommandAccountState::Uninitialized =
            Self::check_approved_command_status(rpc, command_address).await?
        else {
            return Ok(command_address);
        };
        let instruction =
            instructions::initialize_pending_command(&gateway_config_pda, &payer.pubkey(), command)
                .map_err(IncluderError::InitializePendingCommandInstruction)?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        let signature = rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|client_error| {
                IncluderError::InitializePendingCommandTransaction(client_error)
            })?;
        debug!(%signature, "initialized pending command account");

        Ok(command_address)
    }

    /// Fetch a given Solana account, if it exists.
    #[tracing::instrument(skip(rpc), err)]
    async fn fetch_solana_account(
        rpc: &RpcClient,
        account: Pubkey,
    ) -> Result<Option<Account>, IncluderError> {
        rpc.get_account(&account)
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

    /// Checks whether the approved command PDA exists or has been initialized.
    #[tracing::instrument(skip(rpc), err)]
    async fn check_approved_command_status(
        rpc: &RpcClient,
        command_pda: Pubkey,
    ) -> Result<CommandAccountState, IncluderError> {
        let Some(account) = Self::fetch_solana_account(rpc, command_pda).await? else {
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
        rpc: &RpcClient,
        gateway_config_pda: Pubkey,
        execute_data: GatewayExecuteData,
        payer: Arc<Keypair>,
        recent_blockhash: Hash,
    ) -> Result<Vec<Pubkey>, IncluderError> {
        let command_addresses = try_join_all(execute_data.command_batch.commands.into_iter().map(
            |command| {
                Self::initialize_pending_command(
                    rpc,
                    gateway_config_pda,
                    payer.clone(),
                    command,
                    recent_blockhash,
                )
            },
        ))
        .await?;
        debug!("initialized all pending command accounts");
        Ok(command_addresses)
    }

    /// Sends a transaction with the `execute` instruction to the Gateway.
    #[tracing::instrument(skip_all, err)]
    async fn submit_execute_transaction(
        rpc: &RpcClient,
        gateway_address: Pubkey,
        gateway_config_pda: Pubkey,
        execute_data_account: Pubkey,
        command_accounts: &[Pubkey],
        payer: Arc<Keypair>,
        recent_blockhash: Hash,
    ) -> Result<Signature, IncluderError> {
        let instruction = instructions::execute(
            gateway_address,
            execute_data_account,
            gateway_config_pda,
            command_accounts,
        )
        .map_err(IncluderError::ExecuteInstruction)?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        let signature = rpc
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(IncluderError::ExecuteTransaction)?;
        info!(%signature, %execute_data_account, "called `execute`");
        Ok(signature)
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
