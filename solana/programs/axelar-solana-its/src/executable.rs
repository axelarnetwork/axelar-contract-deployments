//! Module that defines the struct used by contracts adhering to the `AxelarInterchainTokenExecutable` interface.

use axelar_executable::AxelarMessagePayload;
use axelar_solana_gateway::state::message_payload::ImmutMessagePayload;
use borsh::{BorshDeserialize, BorshSerialize};
use interchain_token_transfer_gmp::GMPPayload;
use program_utils::BorshPda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;

use crate::assert_valid_its_root_pda;
use crate::state::InterchainTokenService;

/// The index of the first account that is expected to be passed to the
/// destination program. The prepended accounts are:
///
/// 0. [] The Gateway Root Config PDA
/// 1. [signer] The Interchain Token Service Root PDA.
/// 2. [] The Message Payload PDA.
/// 3. [] The token program (spl-token or spl-token-2022).
/// 4. [writable] The token mint.
/// 5. [writable] The Destination Program Associated Token Account.
pub const PROGRAM_ACCOUNTS_START_INDEX: usize = 6;

/// This is the payload that the `executeWithInterchainToken` processor on the destinatoin program
/// must expect
#[derive(Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
#[repr(C)]
pub struct AxelarInterchainTokenExecutablePayload {
    /// The unique message id.
    pub command_id: [u8; 32],

    /// The source chain of the token transfer.
    pub source_chain: String,

    /// The source address of the token transfer.
    pub source_address: String,

    /// The program instruction data.
    pub data: Vec<u8>,

    /// The token ID.
    pub token_id: [u8; 32],

    /// The token (mint) address (Pubkey).
    pub token: [u8; 32],

    /// Amount of tokens being transferred.
    pub amount: u64,
}

/// Axelar Interchain Token Executable command prefix
pub(crate) const AXELAR_INTERCHAIN_TOKEN_EXECUTE: &[u8; 16] = b"axelar-its-exec_";

/// Utility trait to extract the `AxelarInterchainTokenExecutablePayload`
pub trait MaybeAxelarInterchainTokenExecutablePayload {
    /// Try to extract the `AxearlExecutablePayload` from the given payload
    ///
    /// # Errors
    ///
    /// - If the data is not coming from `InterchainTokenService`
    /// - If the data cannot be decoded as a `AxelarInterchainTokenExecutablePayload`
    /// - If list of accounts is different than expected
    /// - If the message account data cannot be borrowed
    /// - If the message account data cannot be decoded as a `GMPPayload`
    fn try_get_axelar_interchain_token_executable_payload<'a>(
        &self,
        accounts: &'a [AccountInfo<'a>],
    ) -> Option<Result<AxelarInterchainTokenExecutablePayload, ProgramError>>;
}

impl MaybeAxelarInterchainTokenExecutablePayload for &[u8] {
    fn try_get_axelar_interchain_token_executable_payload<'a>(
        &self,
        accounts: &'a [AccountInfo<'a>],
    ) -> Option<Result<AxelarInterchainTokenExecutablePayload, ProgramError>> {
        if !self.starts_with(AXELAR_INTERCHAIN_TOKEN_EXECUTE) {
            return None;
        }

        let payload_bytes = self.get(AXELAR_INTERCHAIN_TOKEN_EXECUTE.len()..)?;
        let mut call_data_without_payload: AxelarInterchainTokenExecutablePayload =
            match borsh::from_slice(payload_bytes)
                .map_err(|borsh_error| ProgramError::BorshIoError(borsh_error.to_string()))
            {
                Ok(data) => data,
                Err(err) => return Some(Err(err)),
            };

        let call_data_payload = match extract_interchain_token_execute_call_data(accounts) {
            Ok(data) => data,
            Err(err) => return Some(Err(err)),
        };

        call_data_without_payload.data = call_data_payload;

        Some(Ok(call_data_without_payload))
    }
}

/// Validates accounts and extract extracts the call data for the [`AxelarInterchainTokenExecutablePayload`]
fn extract_interchain_token_execute_call_data<'a>(
    accounts: &'a [AccountInfo<'a>],
) -> Result<Vec<u8>, ProgramError> {
    let (protocol_accounts, program_accounts) = accounts.split_at(PROGRAM_ACCOUNTS_START_INDEX);
    let account_iter = &mut protocol_accounts.iter();
    let gateway_root_account = next_account_info(account_iter)?;
    let signing_pda_account = next_account_info(account_iter)?;
    let message_payload_account = next_account_info(account_iter)?;
    let message_payload_account_data = message_payload_account.try_borrow_data()?;
    let message_payload: ImmutMessagePayload<'_> = (**message_payload_account_data).try_into()?;

    if !signing_pda_account.is_signer {
        msg!(
            "Signing PDA account must be a signer: {}",
            signing_pda_account.key
        );
        return Err(ProgramError::MissingRequiredSignature);
    }

    let its_root_config = InterchainTokenService::load(signing_pda_account)?;
    assert_valid_its_root_pda(
        signing_pda_account,
        gateway_root_account.key,
        its_root_config.bump,
    )?;

    let GMPPayload::ReceiveFromHub(inner) = GMPPayload::decode(message_payload.raw_payload)
        .map_err(|_err| ProgramError::InvalidInstructionData)?
    else {
        msg!("Unsupported GMP payload");
        return Err(ProgramError::InvalidInstructionData);
    };

    let GMPPayload::InterchainTransfer(transfer) =
        GMPPayload::decode(&inner.payload).map_err(|_err| ProgramError::InvalidInstructionData)?
    else {
        msg!("The type of the given ITS message doesn't support call data");
        return Err(ProgramError::InvalidInstructionData);
    };

    let inner_payload = AxelarMessagePayload::decode(transfer.data.as_ref())?;
    if !inner_payload.solana_accounts().eq(program_accounts) {
        msg!("The list of accounts is different than expected");
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(inner_payload.payload_without_accounts().to_vec())
}
