#![deny(missing_docs)]

//! Simple memo program example for the Axelar Gateway on Solana

mod entrypoint;
pub mod processor;
use axelar_executable::axelar_message_primitives::DataPayload;
pub use solana_program;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("Ra5JP1PPSsRP8idQfAWEdSrNCtkN4WTHRRtyxvpKp8C");

/// Build a memo payload instruction
pub fn build_memo<'a>(memo: &'a [u8], pubkeys: &[&Pubkey]) -> DataPayload<'a> {
    let accounts = pubkeys
        .iter()
        .map(|&pubkey| AccountMeta::new_readonly(*pubkey, false))
        .collect::<Vec<_>>();
    DataPayload::new(memo, accounts.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_memo() {
        let signer_pubkey = Pubkey::new_unique();
        let memo = "🐆".as_bytes();
        let instruction = build_memo(memo, &[&signer_pubkey]);
        let payload = instruction.encode();
        let instruction_decoded = DataPayload::decode(&payload);

        assert_eq!(instruction, instruction_decoded);
    }
}
