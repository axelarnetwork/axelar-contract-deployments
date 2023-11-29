//! Instruction types
use std::io::Write;

use slice_iterator::SliceIterator;
use solana_program::instruction::Instruction;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError::{self, *};

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
        id: &'a [u8],
        /// Axelar message payload
        payload: &'a [u8],
        /// Message prof
        proof: &'a [u8],
    },

    /// Represents the `CallContract` Axelar event.
    ///
    ///
    ///
    /// Accounts expected by this instruction:
    /// 0. [] ???
    CallContract {
        /// Message sender.
        sender: Pubkey,
        /// The name of the target blockchain.
        destination_chain: &'a [u8],
        /// The address of the target contract in the destination blockchain.
        destination_contract_address: &'a [u8],
        /// Contract call data.
        payload: &'a [u8],
    },
}

impl<'a> GatewayInstruction<'a> {
    /// Unpacks a byte buffer into a [GatewayInstruction].
    pub fn unpack(input: &'a [u8]) -> Result<Self, GatewayError> {
        let (&tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 => {
                let mut iterator = SliceIterator::new(rest);
                let id = next_slice(&mut iterator)?;
                let payload = next_slice(&mut iterator)?;
                let proof = next_slice(&mut iterator)?;
                GatewayInstruction::Queue { id, payload, proof }
            }
            1 => {
                let (sender, rest) = Self::unpack_pubkey(rest)?;
                let mut iterator = SliceIterator::new(rest);
                let destination_chain = next_slice(&mut iterator)?;
                let destination_contract_address = next_slice(&mut iterator)?;
                let payload = next_slice(&mut iterator)?;
                GatewayInstruction::CallContract {
                    sender,
                    destination_chain,
                    destination_contract_address,
                    payload,
                }
            }
            _ => return Err(InvalidInstruction),
        })
    }

    /// Packs a [GatewayInstruction] into a byte buffer.
    pub fn pack(&self) -> Result<Vec<u8>, GatewayError> {
        let mut buffer: Vec<u8> = Vec::with_capacity(std::mem::size_of::<Self>());
        match *self {
            Self::Queue { id, payload, proof } => {
                buffer.push(0);
                serialize_slices(&[id, payload, proof], &mut buffer)?
            }
            Self::CallContract {
                sender,
                destination_chain,
                destination_contract_address,
                payload,
            } => {
                buffer.push(0);
                serialize_slices(
                    &[
                        &sender.to_bytes(),
                        destination_chain,
                        destination_contract_address,
                        payload,
                    ],
                    &mut buffer,
                )?;
            }
        }
        Ok(buffer)
    }

    fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), GatewayError> {
        if input.len() >= 32 {
            let (key, rest) = input.split_at(32);
            let pk = Pubkey::try_from(key).map_err(|_| InvalidInstruction)?;
            Ok((pk, rest))
        } else {
            Err(InvalidInstruction)
        }
    }
}

/// Creates a [`Queue`] instruction.
// TODO: add arguments for the required accounts.
pub fn queue(
    program_id: Pubkey,
    msg_id: &[u8],
    payload: &[u8],
    proof: &[u8],
) -> Result<Instruction, ProgramError> {
    crate::check_program_account(program_id)?;

    let data = GatewayInstruction::Queue {
        id: msg_id,
        payload,
        proof,
    }
    .pack()?;

    let accounts = vec![
        // TODO: Add required accounts.
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Creates a [`CallContract`] instruction.
// TODO: add arguments for the required accounts.
pub fn call_contract(
    program_id: Pubkey,
    sender: Pubkey,
    destination_chain: &str,
    destination_contract_address: &str,
    payload: &[u8],
) -> Result<Instruction, ProgramError> {
    crate::check_program_account(program_id)?;

    let data = GatewayInstruction::CallContract {
        sender,
        destination_chain: destination_chain.as_bytes(),
        destination_contract_address: destination_contract_address.as_bytes(),
        payload,
    }
    .pack()?;

    let accounts = vec![
        // TODO: Add required accounts.
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

#[cfg(test)]
pub mod tests {

    use arrayref::{array_ref, array_refs};
    use random_array::rand_array;

    use super::*;

    const TAG: usize = 1;
    const SIZE: usize = std::mem::size_of::<u16>();
    const ID: usize = 2;
    const PAYLOAD: usize = 2;
    const PROOF: usize = 2;
    const BUFF: usize = TAG + (SIZE * 3) + ID + PAYLOAD + PROOF;

    #[test]
    fn invalid_discriminant() {
        let input = vec![99]; // invalid tag
        assert_eq!(
            GatewayInstruction::unpack(&input).unwrap_err(),
            InvalidInstruction
        )
    }

    #[test]
    fn unpack_queue() {
        let id = rand_array::<ID>();
        let payload = rand_array::<PAYLOAD>();
        let proof = rand_array::<PROOF>();
        let mut input = vec![0]; // tag
        slice_iterator::serialize_slices(&[&id, &payload, &proof], &mut input).unwrap();

        let instruction = GatewayInstruction::unpack(&input).unwrap();
        match instruction {
            GatewayInstruction::Queue {
                id: unpacked_id,
                payload: unpacked_payload,
                proof: unpacked_proof,
            } => {
                assert_eq!(unpacked_id, id);
                assert_eq!(unpacked_payload, &payload);
                assert_eq!(unpacked_proof, &proof);
            }
            _ => panic!("Wrong instruction"),
        }
    }

    #[test]
    #[allow(clippy::ptr_offset_with_cast)]
    fn pack_queue() {
        let id = &rand_array::<ID>();
        let payload = &rand_array::<PAYLOAD>();
        let proof = &rand_array::<PROOF>();

        let instruction = GatewayInstruction::Queue { id, payload, proof };
        let packed = instruction.pack().unwrap();
        let src = array_ref![packed, 0, BUFF];
        let (tag, id_size, packed_id, payload_size, packed_payload, proof_size, packed_proof) =
            array_refs![src, TAG, SIZE, ID, SIZE, PAYLOAD, SIZE, PROOF];

        let id_size = u16::from_be_bytes(*id_size);
        let payload_size = u16::from_be_bytes(*payload_size);
        let proof_size = u16::from_be_bytes(*proof_size);

        assert_eq!(tag, &[0]);
        assert_eq!(id_size, ID as u16);
        assert_eq!(payload_size, PAYLOAD as u16);
        assert_eq!(proof_size, PROOF as u16);
        assert_eq!(packed_id, id);
        assert_eq!(packed_payload, payload);
        assert_eq!(packed_proof, proof);
    }

    #[test]
    fn round_trip_queue() {
        let id = rand_array::<ID>();
        let original = GatewayInstruction::Queue {
            id: &id,
            payload: &rand_array::<PAYLOAD>(),
            proof: &rand_array::<PROOF>(),
        };
        let packed = original.pack().unwrap();
        let unpacked = GatewayInstruction::unpack(&packed).unwrap();
        assert_eq!(unpacked, original);
    }
}

fn next_slice<'a, I>(iterator: &mut I) -> Result<&'a [u8], GatewayError>
where
    I: Iterator<Item = Result<&'a [u8], slice_iterator::IterationError>>,
{
    iterator
        .next()
        .ok_or(InvalidInstruction)?
        .map_err(|_| InvalidInstruction)
}

fn serialize_slices<W: Write>(src: &[&[u8]], writer: &mut W) -> Result<(), GatewayError> {
    slice_iterator::serialize_slices(src, writer).map_err(|_| ByteSerializationError)
}
