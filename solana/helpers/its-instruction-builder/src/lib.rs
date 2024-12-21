//! Helper crate for building ITS instructions.

use core::ops::Deref;

use axelar_executable::AxelarMessagePayload;
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_its::instructions::ItsGmpInstructionInputs;
use axelar_solana_its::state::token_manager::TokenManager;
use borsh::BorshDeserialize;
use interchain_token_transfer_gmp::GMPPayload;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::clock::Clock;
use solana_sdk::instruction::Instruction;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::sysvar::clock;

/// Creates a [`InterchainTokenServiceInstruction::ItsGmpPayload`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub async fn build_its_gmp_instruction<C>(
    payer: Pubkey,
    gateway_incoming_message_pda: Pubkey,
    gateway_message_payload_pda: Pubkey,
    message: Message,
    abi_payload: Vec<u8>,
    rpc_client: C,
) -> Result<Instruction, ProgramError>
where
    C: Deref<Target = RpcClient> + Send + Sync,
{
    let payload = GMPPayload::decode(&abi_payload).map_err(|_err| ProgramError::InvalidArgument)?;
    ensure_payer_is_not_forwarded(payer, &payload)?;
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&gateway_root_pda);
    let (token_manager_pda, _) = axelar_solana_its::find_token_manager_pda(
        &its_root_pda,
        &payload
            .token_id()
            .map_err(|_err| ProgramError::InvalidArgument)?,
    );

    let clock_account = rpc_client
        .get_account(&clock::id())
        .await
        .map_err(|_err| ProgramError::InvalidAccountData)?;
    let clock: Clock = bincode::deserialize(&clock_account.data)
        .map_err(|_err| ProgramError::InvalidAccountData)?;
    let timestamp = clock.unix_timestamp;

    let (mint, token_program) =
        try_infer_mint_and_program(&token_manager_pda, &payload, rpc_client).await?;

    let inputs = ItsGmpInstructionInputs::builder()
        .payer(payer)
        .incoming_message_pda(gateway_incoming_message_pda)
        .message_payload_pda(gateway_message_payload_pda)
        .message(message)
        .payload(payload)
        .token_program(token_program)
        .mint_opt(mint)
        .timestamp(timestamp)
        .build();

    axelar_solana_its::instructions::its_gmp_payload(inputs)
}

#[async_recursion::async_recursion]
async fn try_infer_mint_and_program<C>(
    token_manager_pda: &Pubkey,
    payload: &GMPPayload,
    rpc_client: C,
) -> Result<(Option<Pubkey>, Pubkey), ProgramError>
where
    C: Deref<Target = RpcClient> + Send + Sync,
{
    match payload {
        GMPPayload::InterchainTransfer(_) => {
            let token_manager_data = rpc_client
                .get_account_data(token_manager_pda)
                .await
                .map_err(|_err| ProgramError::InvalidAccountData)?;

            let token_manager = TokenManager::try_from_slice(&token_manager_data)?;
            let token_mint = Pubkey::new_from_array(
                token_manager
                    .token_address
                    .as_ref()
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidAccountData)?,
            );
            let token_program = rpc_client
                .get_account(&token_mint)
                .await
                .map_err(|_err| ProgramError::InvalidAccountData)?
                .owner;

            Ok((Some(token_mint), token_program))
        }
        GMPPayload::DeployInterchainToken(_) => Ok((None, spl_token_2022::id())),
        GMPPayload::DeployTokenManager(deploy_payload) => {
            let token_mint = axelar_solana_its::state::token_manager::decode_params(
                deploy_payload.params.as_ref(),
            )
            .map(|(_, _, token_mint)| Pubkey::try_from(token_mint.as_ref()))?
            .map_err(|_err| ProgramError::InvalidInstructionData)?;

            let token_program = rpc_client
                .get_account(&token_mint)
                .await
                .map_err(|_err| ProgramError::InvalidAccountData)?
                .owner;

            Ok((Some(token_mint), token_program))
        }
        GMPPayload::SendToHub(_) => return Err(ProgramError::InvalidArgument),
        GMPPayload::ReceiveFromHub(inner) => {
            let inner_payload =
                GMPPayload::decode(&inner.payload).map_err(|_err| ProgramError::InvalidArgument)?;
            try_infer_mint_and_program(token_manager_pda, &inner_payload, rpc_client).await
        }
    }
}

fn ensure_payer_is_not_forwarded(payer: Pubkey, payload: &GMPPayload) -> Result<(), ProgramError> {
    match payload {
        GMPPayload::InterchainTransfer(transfer) => {
            let destination_payload = AxelarMessagePayload::decode(transfer.data.as_ref())?;
            for account in destination_payload.account_meta() {
                if account.pubkey == payer {
                    return Err(ProgramError::InvalidArgument);
                }
            }
        }
        GMPPayload::SendToHub(_) => return Err(ProgramError::InvalidArgument),
        GMPPayload::ReceiveFromHub(inner) => {
            let inner_payload =
                GMPPayload::decode(&inner.payload).map_err(|_err| ProgramError::InvalidArgument)?;
            ensure_payer_is_not_forwarded(payer, &inner_payload)?;
        }
        GMPPayload::DeployInterchainToken(_) | GMPPayload::DeployTokenManager(_) => {}
    }

    Ok(())
}
