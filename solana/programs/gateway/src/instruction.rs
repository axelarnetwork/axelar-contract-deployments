//! Instruction types

use arrayref::{array_ref, array_refs};
use solana_program::instruction::Instruction;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;

const MSG_ID_SIZE: usize = 100;
const PROOF_SIZE: usize = 100;
const PAYLOAD_SIZE: usize = 100;
const QUEUE_INSTRUCTION_SIZE: usize = MSG_ID_SIZE + PROOF_SIZE + PAYLOAD_SIZE;

/// Instructions supported by the gateway program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum GatewayInstruction<'a> {
    /// Receives an Axelar message and initializes a new Message account.
    ///
    /// The `Queue` instruction requires no signers and MUST be
    /// included within the same Transaction as the system program's
    /// `CreateAccount` instruction that creates the account being initialized.
    /// Otherwise another party can acquire ownership of the uninitialized
    /// account.
    ///
    /// The `Registry` account should be initialized before calling this instruction.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The messge to initialize.
    ///   1. `[]` The registry used to validate the proof.
    Queue {
        /// Messge ID
        id: &'a str,
        /// Axelar message payload
        payload: &'a [u8],
        /// Message prof
        proof: &'a [u8],
    },
}

impl<'a> GatewayInstruction<'a> {
    /// Unpacks a byte buffer into a [GatewayInstruction].
    #[allow(clippy::ptr_offset_with_cast)]
    pub fn unpack(input: &'a [u8]) -> Result<Self, GatewayError> {
        use crate::error::GatewayError::InvalidInstruction;
        let (&tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 => {
                let src = array_ref![rest, 0, QUEUE_INSTRUCTION_SIZE];
                let (id, proof, payload) = array_refs![src, MSG_ID_SIZE, PROOF_SIZE, PAYLOAD_SIZE];
                let id = std::str::from_utf8(id).map_err(|_| InvalidInstruction)?;
                Self::Queue { id, payload, proof }
            }
            _ => return Err(GatewayError::InvalidInstruction),
        })
    }

    /// Packs a [GatewayInstruction] into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::with_capacity(std::mem::size_of::<Self>());
        match self {
            &Self::Queue { id, payload, proof } => {
                buf.push(0);
                buf.extend_from_slice(id.as_bytes());
                buf.extend_from_slice(proof);
                buf.extend_from_slice(payload);
            }
        }
        buf
    }
}

/// Creates a `Queue` instruction.
// TODO: add arguments for the required accounts.
pub fn queue(
    gateway_program_id: &Pubkey,
    msg_id: &str,
    payload: &[u8],
    proof: &[u8],
) -> Result<Instruction, ProgramError> {
    crate::check_program_account(gateway_program_id)?;

    let data = GatewayInstruction::Queue {
        id: msg_id,
        payload,
        proof,
    }
    .pack();

    let accounts = vec![
        // TODO: Add required accounts.
    ];

    Ok(Instruction {
        program_id: *gateway_program_id,
        accounts,
        data,
    })
}

#[cfg(test)]
pub mod tests {

    use test_utilities::{rand_array, rand_str};

    use super::*;
    use crate::error::GatewayError;

    #[test]
    fn unpack() {
        let id = rand_str(MSG_ID_SIZE);
        let payload = &rand_array::<PAYLOAD_SIZE>();
        let proof = &rand_array::<PROOF_SIZE>();
        let mut input = vec![0]; // tag for Queue instruction
        input.extend_from_slice(id.as_bytes());
        input.extend_from_slice(proof);
        input.extend_from_slice(payload);

        let instruction = GatewayInstruction::unpack(&input).unwrap();
        match instruction {
            GatewayInstruction::Queue {
                id: unpacked_id,
                payload: unpacked_payload,
                proof: unpacked_proof,
            } => {
                assert_eq!(unpacked_id, id);
                assert_eq!(unpacked_payload, &payload[..]);
                assert_eq!(unpacked_proof, &proof[..]);
            }
        }
    }

    #[test]
    fn invalid_discriminant() {
        let input = vec![99]; // invalid tag
        assert_eq!(
            GatewayInstruction::unpack(&input).unwrap_err(),
            GatewayError::InvalidInstruction
        )
    }

    #[test]
    #[allow(clippy::ptr_offset_with_cast)]
    fn pack() {
        let id = rand_str(MSG_ID_SIZE);
        let payload = &rand_array::<PAYLOAD_SIZE>();
        let proof = &rand_array::<PROOF_SIZE>();

        let instruction = GatewayInstruction::Queue {
            id: id.as_str(),
            payload,
            proof,
        };
        let packed = instruction.pack();

        let packed = array_ref![packed, 0, QUEUE_INSTRUCTION_SIZE + 1];
        let (packed_tag, packed_id, packed_proof, packed_payload) =
            array_refs![packed, 1, MSG_ID_SIZE, PROOF_SIZE, PAYLOAD_SIZE];
        let packed_id = std::str::from_utf8(packed_id).expect("valid utf-8 message id");
        assert_eq!(packed_tag, &[0]);
        assert_eq!(packed_id, id);
        assert_eq!(packed_proof, proof);
        assert_eq!(packed_payload, payload);
    }

    #[test]
    fn round_trip() {
        let id = rand_str(MSG_ID_SIZE);
        let original = GatewayInstruction::Queue {
            id: &id,
            payload: &rand_array::<PAYLOAD_SIZE>(),
            proof: &rand_array::<PROOF_SIZE>(),
        };
        let packed = original.pack();
        let unpacked = GatewayInstruction::unpack(&packed).unwrap();
        assert_eq!(unpacked, original);
    }
}
