//! Types used for logging messages.

use base64::engine::general_purpose;
use base64::Engine as _;
use borsh::{self, BorshDeserialize, BorshSerialize};
use gateway::types::PubkeyWrapper;
use solana_program::log::sol_log_data;
use solana_program::program_error::ProgramError;
use token_manager::TokenManagerType;

/// Interchain Token Service logs.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum InterchainTokenServiceEvent {
    /// Emitted for token id claimed.
    InterchainTokenIdClaimed {
        /// The interchain token id.
        token_id: [u8; 32],
        /// The message sender.
        deployer: PubkeyWrapper,
        /// The salt to be used during deployment.
        salt: [u8; 32],
    },
    /// Emitted for token manager deployment starting.
    TokenManagerDeploymentStarted {
        /// The interchain token id.
        token_id: [u8; 32],
        /// The chain where the token manager will be deployed.
        destination_chain: Vec<u8>,
        /// The type of token manager to be deployed.
        token_manager_type: TokenManagerType,
        /// The additional parameters for the token manager deployment.
        params: Vec<u8>,
    },
}

impl InterchainTokenServiceEvent {
    /// Emits the log for this event.
    pub fn emit(&self) -> Result<(), ProgramError> {
        let serialized = borsh::to_vec(self)?;
        sol_log_data(&[&serialized]);
        Ok(())
    }

    /// Try to parse a [`InterchainTokenServiceEvent`] out of a Solana program
    /// log line.
    pub fn parse_log<T: AsRef<str>>(log: T) -> Option<Self> {
        let cleaned_input = log
            .as_ref()
            .trim()
            .trim_start_matches("Program data:")
            .split_whitespace()
            .flat_map(decode_base64)
            .next()?;
        borsh::from_slice(&cleaned_input).ok()
    }
}

#[inline]
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    general_purpose::STANDARD.decode(input).ok()
}

/// Emit a [`InterchainTokenIdClaimed`].
pub fn emit_interchain_token_id_claimed_event(
    token_id: [u8; 32],
    deployer: PubkeyWrapper,
    salt: [u8; 32],
) -> Result<(), ProgramError> {
    let event = InterchainTokenServiceEvent::InterchainTokenIdClaimed {
        token_id,
        deployer,
        salt,
    };
    event.emit()
}

/// Emit a [`TokenManagerDeploymentStarted`].
pub fn emit_token_manager_deployment_started_event(
    token_id: [u8; 32],
    destination_chain: Vec<u8>,
    token_manager_type: TokenManagerType,
    params: Vec<u8>,
) -> Result<(), ProgramError> {
    let event = InterchainTokenServiceEvent::TokenManagerDeploymentStarted {
        token_id,
        destination_chain,
        token_manager_type,
        params,
    };
    event.emit()
}
