//! Helper crate for building ITS instructions.

use core::ops::Deref;

use axelar_rkyv_encoding::types::GmpMetadata;
use axelar_solana_its::instructions::{Bumps, ItsGmpInstructionInputs};
use axelar_solana_its::state::token_manager::ArchivedTokenManager;
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
    gateway_approved_message_pda: Pubkey,
    gateway_root_pda: Pubkey,
    gmp_metadata: GmpMetadata,
    abi_payload: Vec<u8>,
    rpc_client: C,
) -> Result<Instruction, ProgramError>
where
    C: Deref<Target = RpcClient>,
{
    let payload = GMPPayload::decode(&abi_payload).map_err(|_err| ProgramError::InvalidArgument)?;
    let (its_root_pda, its_root_pda_bump) = axelar_solana_its::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, interchain_token_pda_bump) =
        axelar_solana_its::find_interchain_token_pda(
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

    let (token_manager_pda, token_manager_pda_bump) =
        axelar_solana_its::find_token_manager_pda(&interchain_token_pda);
    let (mint, token_program) =
        try_infer_mint_and_program(&token_manager_pda, &payload, rpc_client).await?;

    let bumps = Some(Bumps {
        its_root_pda_bump,
        interchain_token_pda_bump,
        token_manager_pda_bump,
        ..Default::default()
    });

    let inputs = ItsGmpInstructionInputs::builder()
        .payer(payer)
        .gateway_approved_message_pda(gateway_approved_message_pda)
        .gateway_root_pda(gateway_root_pda)
        .gmp_metadata(gmp_metadata)
        .payload(payload)
        .token_program(token_program)
        .mint_opt(mint)
        .bumps_opt(bumps)
        .timestamp(timestamp)
        .build();

    axelar_solana_its::instructions::its_gmp_payload(inputs)
}

#[async_recursion::async_recursion(?Send)]
async fn try_infer_mint_and_program<C>(
    token_manager_pda: &Pubkey,
    payload: &GMPPayload,
    rpc_client: C,
) -> Result<(Option<Pubkey>, Pubkey), ProgramError>
where
    C: Deref<Target = RpcClient>,
{
    match payload {
        GMPPayload::InterchainTransfer(_) => {
            let token_manager_data = rpc_client
                .get_account_data(token_manager_pda)
                .await
                .map_err(|_err| ProgramError::InvalidAccountData)?;

            let token_manager = ArchivedTokenManager::from_bytes(&token_manager_data);
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
            .map(|(_, token_mint)| Pubkey::try_from(token_mint.as_ref()))?
            .map_err(|_err| ProgramError::InvalidInstructionData)?;

            let token_program = rpc_client
                .get_account(&token_mint)
                .await
                .map_err(|_err| ProgramError::InvalidAccountData)?
                .owner;

            Ok((Some(token_mint), token_program))
        }
        GMPPayload::SendToHub(inner) => {
            let inner_payload =
                GMPPayload::decode(&inner.payload).map_err(|_err| ProgramError::InvalidArgument)?;
            try_infer_mint_and_program(token_manager_pda, &inner_payload, rpc_client).await
        }
        GMPPayload::ReceiveFromHub(inner) => {
            let inner_payload =
                GMPPayload::decode(&inner.payload).map_err(|_err| ProgramError::InvalidArgument)?;
            try_infer_mint_and_program(token_manager_pda, &inner_payload, rpc_client).await
        }
    }
}
