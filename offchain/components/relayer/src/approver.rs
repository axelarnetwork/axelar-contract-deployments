use std::sync::Arc;

use axelar_executable::axelar_message_primitives::command::{
    ApproveContractCallCommand, DecodedCommand,
};
use axelar_executable::axelar_message_primitives::DataPayload;
use gmp_gateway::error::GatewayError;
use gmp_gateway::state::GatewayApprovedCommand;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use thiserror::Error;
use tonic::transport::Channel;
use tracing::{error, info, warn};

use self::block_messages::BlockMessages;
use crate::amplifier_api::amplifier_client::AmplifierClient;
use crate::amplifier_api::{
    GetPayloadRequest, SubscribeToApprovalsRequest, SubscribeToApprovalsResponse,
};
use crate::config::SOLANA_CHAIN_NAME;
use crate::state::State;

mod block_messages;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ApproverError {
    #[error("Failed to subscribe for approvals from Axelar")]
    SubForApprovals(tonic::Status),
    #[error("Failed to create initialize execute data instruction - {0}")]
    CreateInitExecDataIx(ProgramError),
    #[error("Failed to create execute instruction for the gateway - {0}")]
    CreateExecuteIx(ProgramError),
    #[error("Failed to send approve tx to gateway - {0}")]
    SendAndConfirm(solana_client::client_error::ClientError),
    #[error("Failed to subscribe for approvals from Axelar - {0}")]
    ExecuteDataDecode(GatewayError),
    #[error("Failed to get latest blockhash from Solana - {0}")]
    GetLatestBlockhash(solana_client::client_error::ClientError),
    #[error("Failed to get payload from Axelar - {0}")]
    GetPayload(tonic::Status),
    #[error("Failed to decode payload - {0}")]
    DecodePayload(bcs::Error),
    #[error("Failed to fetch proof from Amplifier API stream with status - {0}")]
    GetProofFromStream(tonic::Status),
    #[error("State error - {0}")]
    State(#[from] sqlx::Error),
    #[error(transparent)]
    PayloadError(#[from] axelar_executable::axelar_message_primitives::PayloadError),
}

/// Listens for approved messages (signed proofs) coming from the Axelar
/// blockchain.
///
/// Those will be payloads sent from other blockchains,
/// which pass through axelar and are sent to Solana.
#[allow(dead_code)]
pub struct Approver {
    amplifier_rpc_client: AmplifierClient<Channel>,
    solana_rpc_client: Arc<RpcClient>,
    payer_keypair: Arc<Keypair>,
    state: State,
    gmp_gateway_root_config_pda: Pubkey,
}

impl Approver {
    /// Create a new sentinel, watching for messages coming from Axelar
    pub fn new(
        amplifier_rpc_client: AmplifierClient<Channel>,
        solana_rpc_client: Arc<RpcClient>,
        payer_keypair: Arc<Keypair>,
        state: State,
    ) -> Self {
        let (gmp_gateway_root_config_pda, _) = gmp_gateway::get_gateway_root_config_pda();
        Self {
            amplifier_rpc_client,
            solana_rpc_client,
            payer_keypair,
            state,
            gmp_gateway_root_config_pda,
        }
    }

    #[tracing::instrument(level = "info", skip_all)]
    async fn initialize_execute_data_account(
        &self,
        ix: solana_sdk::instruction::Instruction,
    ) -> Result<(), ApproverError> {
        let recent_blockhash = self
            .solana_rpc_client
            .get_latest_blockhash()
            .await
            .map_err(ApproverError::GetLatestBlockhash)?;

        // TODO: This will send the init exec data tx async
        // let tx = TxType::InitExecDataAccount(Transaction::new_signed_with_payer(
        //     &[ix],
        //     Some(&self.payer_keypair.pubkey()),
        //     &[&self.payer_keypair],
        //     recent_blockhash,
        // ));

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer_keypair.pubkey()),
            &[&self.payer_keypair],
            recent_blockhash,
        );

        let tx_resp = self
            .solana_rpc_client
            .send_and_confirm_transaction(&tx)
            .await
            .map_err(ApproverError::SendAndConfirm)?;

        info!("Init execute data account TX SIGNATURE {}", tx_resp);

        // TODO: This will send the init exec data tx async
        // let broadcast_result = self.broadcast_sender.send(tx);
        // if let Err(err) = broadcast_result {
        //     error!(%err, "failed to send 'init execute data account' for
        // broadcasting"); }

        Ok(())
    }

    #[tracing::instrument(level = "info", skip_all)]
    async fn handle_proof(&self, proof: SubscribeToApprovalsResponse) -> Result<(), ApproverError> {
        // Decode execute_data, get msg ids and construct accounts

        let (ix, decoded_execute_data) = gmp_gateway::instructions::initialize_execute_data(
            self.payer_keypair.pubkey(),
            self.gmp_gateway_root_config_pda,
            proof.execute_data,
        )
        .map_err(ApproverError::CreateInitExecDataIx)?;

        // Construct msg accounts for the Solana tx
        // TODO: Prover should not include more than X msgs in the batch. We can also do
        // that check here if needed?
        let message_accounts: Vec<Pubkey> = decoded_execute_data
            .command_batch
            .commands
            .iter()
            .map(|command| {
                GatewayApprovedCommand::pda(&self.gmp_gateway_root_config_pda, command).0
            })
            .collect();

        // Construct the execute_data account
        let execute_data_account = {
            let (execute_data_pda, _bump, _seeds) =
                decoded_execute_data.pda(&self.gmp_gateway_root_config_pda);
            self.initialize_execute_data_account(ix).await?;
            execute_data_pda
        };

        // Construct the execute (approve) Solana instruction to the Axelar gateway on
        // solana
        let approve_ix = gmp_gateway::instructions::execute(
            gmp_gateway::ID,
            execute_data_account,
            self.gmp_gateway_root_config_pda,
            &message_accounts,
        )
        .map_err(ApproverError::CreateExecuteIx)?;

        let latest_blockhash = self
            .solana_rpc_client
            .get_latest_blockhash()
            .await
            .map_err(ApproverError::GetLatestBlockhash)?;

        let transaction = Transaction::new_signed_with_payer(
            &[approve_ix],
            Some(&self.payer_keypair.pubkey()),
            &[&self.payer_keypair],
            latest_blockhash,
        );

        // Send execute (approve) tx for broadcasting
        let _tx_resp = self
            .solana_rpc_client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(ApproverError::SendAndConfirm)?;

        for (decoded_command, approved_command_pda) in decoded_execute_data
            .command_batch
            .commands
            .into_iter()
            .zip(message_accounts)
        {
            match decoded_command {
                DecodedCommand::ApproveContractCall(command) => {
                    self.execute_destination_program(command, approved_command_pda)
                        .await?;
                }
                DecodedCommand::TransferOperatorship(_command) => {
                    // no-op because this already gets executed in the
                    // gatway.execute call
                }
            }
        }

        Ok(())
    }

    /// Execute a command from Axelar by calling the destination program!
    async fn execute_destination_program(
        &self,
        message: ApproveContractCallCommand,
        approved_message_pda: Pubkey,
    ) -> Result<(), ApproverError> {
        // Get the actual payload for that hash from Axelar
        let payload_bytes = self
            .amplifier_rpc_client
            .clone()
            .get_payload(GetPayloadRequest {
                hash: message.payload_hash.to_vec(),
            })
            .await
            .map_err(ApproverError::GetPayload)?;
        // sanity check: decoding of the payload - no point to send & pay for a tx if we
        // can check it here
        let payload_bytes = payload_bytes.into_inner().payload;
        let payload = DataPayload::decode(payload_bytes.as_ref())?;

        // Decode the payload as a solana Instruction type
        let destinatoin_program = message.destination_program;
        let ix = axelar_executable::construct_axelar_executable_ix(
            message,
            payload.encode()?,
            approved_message_pda,
            self.gmp_gateway_root_config_pda,
        )
        .unwrap(); // todo handle error

        // mostly for accounting purposes
        if ix.program_id != destinatoin_program.0 {
            warn!("program_id provided from the decoded instruction doesn't match with the destination_address passed in the Axelar message; ix - {}; msg - {}", ix.program_id, destinatoin_program.0);

            // TODO: Arguable if we should skip sending the tx or just pick
            // what's in the instruction as the correct one by
            // default
        }

        // Craft an execute tx and send to broadcast as TxType::Execute
        let latest_blockhash = self
            .solana_rpc_client
            .get_latest_blockhash()
            .await
            .map_err(ApproverError::GetLatestBlockhash)?;

        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer_keypair.pubkey()),
            &[&self.payer_keypair],
            latest_blockhash,
        );

        self.solana_rpc_client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(ApproverError::SendAndConfirm)
            .map(|_| ())
    }

    pub async fn run(&mut self) -> Result<(), ApproverError> {
        let start_height = self.state.get_axelar_block_height().await?;

        // Init the stream of proofs coming from the Amplifier API
        let mut stream = self
            .amplifier_rpc_client
            .subscribe_to_approvals(SubscribeToApprovalsRequest {
                chains: vec![SOLANA_CHAIN_NAME.into()],
                start_height: Some(start_height as u64),
            })
            .await
            .map_err(ApproverError::SubForApprovals)?
            .into_inner();

        let mut block_messages: BlockMessages = BlockMessages::new(0);

        while let Some(axl_proof) = stream.message().await.unwrap() {
            if let Some(messages) = block_messages.indicate_height(axl_proof.block_height) {
                self.handle_block_messages(messages).await?;

                self.state
                    // FIXME: this cast might be problematic
                    .update_axelar_block_height(axl_proof.block_height as i64)
                    .await?;
            }

            block_messages.push(axl_proof);
        }

        Ok(())
    }

    async fn handle_block_messages(
        &self,
        block_messages: Vec<SubscribeToApprovalsResponse>,
    ) -> Result<(), ApproverError> {
        for axl_proof in block_messages {
            let proof_handle_resp = self.handle_proof(axl_proof).await;
            if let Err(error) = proof_handle_resp {
                error!(%error);
            }
        }

        Ok(())
    }
}
