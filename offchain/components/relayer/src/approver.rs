use amplifier_api::axl_rpc::{
    axelar_rpc_client::AxelarRpcClient, GetPayloadRequest, SubscribeToApprovalsRequest,
    SubscribeToApprovalsResponse,
};
use gmp_gateway::{accounts::GatewayApprovedMessage, error::GatewayError};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use solana_sdk::{signature::Keypair, signature::Signer, transaction::Transaction};
use std::sync::Arc;
use thiserror::Error;
use tonic::transport::Channel;
use tracing::{error, info, warn};

#[derive(Debug, Error)]
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
}

/// Listens for approved messages (signed proofs) coming from the Axelar blockchain.
///
/// Those will be payloads sent from other blockchains,
/// which pass through axelar and are sent to Solana.

pub struct Approver {
    source_chain: String,
    amplifier_rpc_client: AxelarRpcClient<Channel>,
    solana_rpc_client: Arc<RpcClient>,
    payer_keypair: Arc<Keypair>,
}

impl Approver {
    /// Create a new sentinel, watching for messages coming from Axelar
    pub fn new(
        source_chain: String,
        amplifier_rpc_client: AxelarRpcClient<Channel>,
        solana_rpc_client: Arc<RpcClient>,
        payer_keypair: Arc<Keypair>,
    ) -> Self {
        Self {
            source_chain,
            amplifier_rpc_client,
            solana_rpc_client,
            payer_keypair,
        }
    }

    #[tracing::instrument(level = "info", skip_all)]
    async fn initialize_execute_data_account(
        &self,
        pda: Pubkey,
        execute_data: gmp_gateway::accounts::GatewayExecuteData,
    ) -> Result<(), ApproverError> {
        let recent_blockhash = self
            .solana_rpc_client
            .get_latest_blockhash()
            .await
            .map_err(ApproverError::GetLatestBlockhash)?;

        let ix = gmp_gateway::instructions::initialize_execute_data(
            self.payer_keypair.pubkey(),
            pda,
            execute_data.clone(),
        )
        .map_err(ApproverError::CreateInitExecDataIx)?;

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
        //     error!(%err, "failed to send 'init execute data account' for broadcasting");
        // }

        Ok(())
    }

    #[tracing::instrument(level = "info", skip_all)]
    async fn handle_proof(&self, proof: SubscribeToApprovalsResponse) -> Result<(), ApproverError> {
        // Decode execute_data, get msg ids and construct accounts
        let execute_data = gmp_gateway::accounts::GatewayExecuteData::new(proof.execute_data);
        let (_proof, command_batch) = execute_data
            .decode()
            .map_err(ApproverError::ExecuteDataDecode)?;

        // Construct msg accounts for the Solana tx
        let message_accounts: Vec<Pubkey> =
            // TODO: Prover should not include more than X msgs in the batch. We can also do that
            // check here if needed?
            command_batch
                .commands
                .iter()
                .map(GatewayApprovedMessage::pda_from_decoded_command)
                .collect();

        // Construct the execute_data account
        let execute_data_account = {
            let (execute_data_pda, _bump, _seeds) = execute_data.pda();
            self.initialize_execute_data_account(execute_data_pda, execute_data)
                .await?;
            execute_data_pda
        };

        // Construct the execute (approve) Solana instruction to the Axelar gateway on solana
        let approve_ix = gmp_gateway::instructions::execute(
            gmp_gateway::ID,
            execute_data_account,
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

        for decoded_command in command_batch.commands {
            self.execute(decoded_command.message).await?
        }

        Ok(())
    }

    async fn execute(
        &self,
        message: gmp_gateway::types::execute_data_decoder::DecodedMessage,
    ) -> Result<(), ApproverError> {
        let destination_addr = Pubkey::from(message.destination_address);

        // Get the actual payload for that hash from Axelar
        let payload_bytes = self
            .amplifier_rpc_client
            .clone()
            .get_payload(GetPayloadRequest {
                hash: message.payload_hash.to_vec(),
            })
            .await
            .map_err(ApproverError::GetPayload)?;

        // Decode the payload as a solana Instruction type
        let ix: solana_program::instruction::Instruction =
            bcs::from_bytes(&payload_bytes.into_inner().payload)
                .map_err(ApproverError::DecodePayload)?;
        // ix.accounts
        //     .insert(0, AccountMeta::new(s..payer_keypair.pubkey(), true));

        // mostly for accounting purposes
        if ix.program_id != destination_addr {
            warn!("program_id provided from the decoded instruction doesn't match with the destination_address passed in the Axelar message; ix - {}; msg - {}", ix.program_id, destination_addr);

            // TODO: Arguable if we should skip sending the tx or just pick what's in
            // the instruction as the correct one by default
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

    /// Listens for a signed proof, coming from the Axelar blockchain
    /// and sends it directly to the Axelar gateway on the solana blockchain in the for of an
    /// execute transaction.
    /// Execute in the gateway actually approves the payload.
    pub async fn run(&mut self) -> Result<(), ApproverError> {
        // Init the stream of proofs coming from the Amplifier API
        let mut stream = self
            .amplifier_rpc_client
            .subscribe_to_approvals(SubscribeToApprovalsRequest {
                chain: self.source_chain.clone(),
                start_height: None, // TODO: Get from state file/db/whatever
            })
            .await
            .map_err(ApproverError::SubForApprovals)?
            .into_inner();

        while let Some(axl_proof) = stream
            .message()
            .await
            .map_err(ApproverError::GetProofFromStream)?
        {
            println!("PROOF RECEIVED");
            let proof_handle_resp = self.handle_proof(axl_proof).await;
            if let Err(error) = proof_handle_resp {
                error!(%error);
            }
        }

        Ok(())
    }
}
