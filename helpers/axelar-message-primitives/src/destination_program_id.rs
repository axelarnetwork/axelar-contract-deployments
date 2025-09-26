use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Newtype for a destination address.
/// This is the program ID of the destination program.
#[derive(Debug, PartialEq, Eq, Copy, Clone, BorshSerialize, BorshDeserialize)]
pub struct DestinationProgramId(pub Pubkey);

impl From<Pubkey> for DestinationProgramId {
    fn from(pubkey: Pubkey) -> Self {
        DestinationProgramId(pubkey)
    }
}

impl DestinationProgramId {
    /// Returns the signing PDA for this destination address and message ID.
    ///
    /// Only the destination program is allowed to sign the message for
    /// validating that a message is being executed - this is reference to
    /// gateway.validateMessage.
    pub fn signing_pda(&self, command_id: &[u8; 32]) -> (Pubkey, u8) {
        // This corresponds to `axelar_solana_gateway::seed_prefixes::VALIDATE_MESSAGE_SIGNING_SEED`
        Pubkey::find_program_address(&[b"gtw-validate-msg", command_id], &self.0)
    }
}
