use axelar_executable::axelar_message_primitives::command::DecodeError;
use solana_client::client_error::ClientError;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;

/// Errors that can happen within the Solana Includer module.
#[derive(Debug, thiserror::Error)]
pub(super) enum IncluderError {
    /// Used when the payload of a Message fails to be decoded.
    #[error("failed to decode Message payload: {0}")]
    Decode(#[from] DecodeError),

    /// Used when an 'initialize_pending_command' instruction can't be
    /// constructed.
    #[error("failed to create an initialize_pending_command instruction: {0}")]
    InitializePendingCommandInstruction(#[source] ProgramError),

    /// Used when an 'initialize_pending_command' instruction can't be
    /// constructed.
    #[error("failed to create an 'initialize_execute_data' instruction: {0}")]
    InitializeExecuteDataInstruction(#[source] ProgramError),

    /// Used when an `ApproveMessages` instruction can't be constructed.
    #[error("failed to create an `ApproveMessages` instruction: {0}")]
    ApproveMessagesInstruction(#[source] ProgramError),

    /// Used when a `RotateSigners` instruction can't be constructed.
    #[error("failed to create a `RotateSigners` instruction: {0}")]
    RotateSignersInstruction(#[source] ProgramError),

    /// Used when we fail to submit a transaction to initialize a pending
    /// command's PDA.
    #[error("failed to submit an initialize_pending_command transaction")]
    InitializePendingCommandTransaction(#[source] ClientError),

    /// Used when we fail to submit a transaction to initialize the
    /// execute_data.
    #[error("failed to submit an initialize_execute_data transaction")]
    InitializeExecuteDataTransaction(#[source] ClientError),

    /// Used when we fail to submit an `execute` transaction.
    #[error("failed to submit an execute transaction")]
    ExecuteTransaction(#[source] ClientError),

    /// Used when we fail to verify if an account exists before attempt to
    /// initialize it.
    #[error("failed to check if an account was initialized")]
    AccountPreInitializationCheck {
        #[source]
        error: ClientError,
        account: Pubkey,
    },

    /// Used when we fail to deserialize a `GatewayApprovedCommand`
    #[error("failed to deserialize an approved command account")]
    ApprovedCommandDeserialization(#[source] std::io::Error),

    /// Used when the Solana RPC fails to return a recent block hash to be used
    /// as a transaction parameter.
    #[error("failed to obtain the latest block hash from Solana RPC")]
    LatestBlockHash(#[source] ClientError),

    /// Used when the channel is closed by the other side.
    #[error("the channel has been closed")]
    ChannelClosed,

    /// Used when the Solana Includer receives the cancellation signal.
    #[error("received the cancellation signal")]
    Cancelled,

    /// Used when converting Axelar block height (u64) for saving it into the
    /// state fails.
    #[error("Block height too big to fit i64::MAX: {0}")]
    BlockHeightOverflow(#[from] std::num::TryFromIntError),

    /// Used when persisting the latest known block height.
    #[error("Failed to persist the latest block height: {0}")]
    State(#[from] sqlx::Error),

    /// Used when an unexpected number of command accounts were found when
    /// handling a `RotateSigners` instruction.
    ///
    /// This should rarely happen as this invariant is enforced by the Gateway
    /// decoding functions, but we check it here just in case.
    #[error(
        "Expected a single command account for the `RotateSigners` instructions but found {length}"
    )]
    MissingOrMultipleRotateSignersCommandAccounts { length: usize },

    /// Used when a command batch has an empty command list.
    ///
    /// This should rarely happen as this invariant is enforced by the Gateway
    /// decoding functions, but we check it here just in case.
    #[error("Empty command account list")]
    EmptyCommandsList,
}

impl IncluderError {
    pub fn is_fatal(&self) -> bool {
        use IncluderError::*;
        matches!(
            self,
            ChannelClosed | Cancelled | BlockHeightOverflow(_) | State(_)
        )
    }
}
