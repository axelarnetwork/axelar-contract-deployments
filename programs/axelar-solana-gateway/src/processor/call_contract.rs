use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::error::GatewayError;
use crate::events::CallContractEvent;
use crate::{assert_initialized_and_valid_gateway_root_pda, create_call_contract_signing_pda};

impl Processor {
    /// This function initializes a cross-chain message by emitting an event containing the call details.
    ///
    /// The message can then be picked up by off-chain components for
    /// cross-chain delivery.
    ///
    /// It requires a valid signing PDA & signing PDA bump to be provided for verifying the
    /// authenticity of the call.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Required accounts are not provided
    /// * Gateway root PDA is not properly initialized
    /// * Gateway root PDA's bump seed is invalid
    /// * Sender is not a signer
    ///
    /// Returns [`GatewayError`] if:
    /// * Gateway configuration data is invalid (`BytemuckDataLenInvalid`)
    ///
    /// # Events
    ///
    /// Emits a `CALL_CONTRACT` event with the following data:
    /// * Sender's public key
    /// * Keccak256 hash of the payload
    /// * Destination chain identifier
    /// * Destination contract address
    /// * Raw payload data
    pub fn process_call_contract(
        _program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        destination_chain: &str,
        destination_contract_address: &str,
        payload: Vec<u8>,
        signing_pda_bump: u8,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let sender = next_account_info(accounts_iter)?;
        let sender_signing_pda = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        event_cpi_accounts!(accounts_iter);

        // Check: Gateway Root PDA is initialized.
        assert_initialized_and_valid_gateway_root_pda(gateway_root_pda)?;

        if sender.is_signer {
            // Direct signer, so not a program, continue
        } else {
            // Case of a program, so a valid signing PDA must be provided
            let Ok(expected_signing_pda) =
                create_call_contract_signing_pda(*sender.key, signing_pda_bump)
            else {
                solana_program::msg!(
                    "Invalid call: sender must be a direct signer or a valid signing PDA must be provided",
                );
                return Err(GatewayError::CallerNotSigner.into());
            };

            if &expected_signing_pda != sender_signing_pda.key {
                // Signing PDA mismatch
                solana_program::msg!("Invalid call: a valid signing PDA must be provided",);
                return Err(GatewayError::InvalidSigningPDA.into());
            }

            if !sender_signing_pda.is_signer {
                // Signing PDA is correct but not a signer
                solana_program::msg!("Signing PDA must be a signer");
                return Err(GatewayError::CallerNotSigner.into());
            }

            // A valid signing PDA was provided and it's a signer, continue
        }

        // compute the payload hash
        let payload_hash = solana_program::keccak::hash(&payload).to_bytes();

        emit_cpi!(CallContractEvent {
            sender: *sender.key,
            payload_hash,
            destination_chain: destination_chain.to_string(),
            destination_contract_address: destination_contract_address.to_string(),
            payload,
        });

        Ok(())
    }
}
