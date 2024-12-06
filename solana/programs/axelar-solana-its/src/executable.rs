//! Module that defines the struct used by contracts adhering to the `AxelarInterchainTokenExecutable` interface.

use program_utils::StorableArchive;
use rkyv::bytecheck::{self, CheckBytes};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;

use crate::assert_valid_its_root_pda;
use crate::state::InterchainTokenService;

/// The index of the first account that is expected to be passed to the
/// destination program. The prepended accounts are:
///
/// 0. [] The Gateway Root Config PDA
/// 1. [signer] The Interchain Token Service Root PDA.
/// 2. [] The token program (spl-token or spl-token-2022).
/// 3. [writable] The token mint.
/// 4. [writable] The Destination Program Associated Token Account.
pub const PROGRAM_ACCOUNTS_START_INDEX: usize = 5;

/// This is the payload that the `executeWithInterchainToken` processor on the destinatoin program
/// must expect
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
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
    fn try_get_axelar_interchain_token_executable_payload(
        &self,
    ) -> Option<Result<&ArchivedAxelarInterchainTokenExecutablePayload, ProgramError>>;
}

impl MaybeAxelarInterchainTokenExecutablePayload for &[u8] {
    fn try_get_axelar_interchain_token_executable_payload(
        &self,
    ) -> Option<Result<&ArchivedAxelarInterchainTokenExecutablePayload, ProgramError>> {
        let first_16_bytes = self.get(0..AXELAR_INTERCHAIN_TOKEN_EXECUTE.len())?;
        if first_16_bytes != AXELAR_INTERCHAIN_TOKEN_EXECUTE {
            solana_program::msg!("Invalid instruction data: {:?}", first_16_bytes);
            return None;
        }
        let all_other_bytes = self.get(AXELAR_INTERCHAIN_TOKEN_EXECUTE.len()..)?;
        let result =
            rkyv::check_archived_root::<AxelarInterchainTokenExecutablePayload>(all_other_bytes)
                .map_err(|_err| ProgramError::InvalidInstructionData);
        Some(result)
    }
}

/// Used to validate that the caller is the Interchain Token Service
///
/// # Errors
///
/// If the caller is not the Interchain Token Service
pub fn validate_interchain_token_execute_call<'a>(
    accounts: &'a [AccountInfo<'a>],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let gateway_root_account = next_account_info(account_iter)?;
    let signing_pda_account = next_account_info(account_iter)?;

    if !signing_pda_account.is_signer {
        msg!(
            "Signing PDA account must be a signer: {}",
            signing_pda_account.key
        );
        return Err(ProgramError::MissingRequiredSignature);
    }

    let its_root_config = InterchainTokenService::load_readonly(&crate::id(), signing_pda_account)?;
    assert_valid_its_root_pda(
        signing_pda_account,
        gateway_root_account.key,
        its_root_config.bump,
    )?;

    Ok(())
}
