//! Instruction types
use std::io::Write;

use slice_iterator::SliceIterator;
use solana_program::instruction::Instruction;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError::{self, *};

/// Instructions supported by the gateway program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq)]
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
        /// Contract call data.
        payload_hash: [u8; 32],
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
                let (payload_hash, _rest) = Self::unpack_payload_hash(iterator.rest())?;
                GatewayInstruction::CallContract {
                    sender,
                    destination_chain,
                    destination_contract_address,
                    payload,
                    payload_hash,
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
                payload_hash,
            } => {
                buffer.push(1);
                buffer.extend(sender.as_ref());
                serialize_slices(
                    &[destination_chain, destination_contract_address, payload],
                    &mut buffer,
                )?;
                buffer.extend(&payload_hash);
            }
        }
        Ok(buffer)
    }

    fn unpack_payload_hash(input: &[u8]) -> Result<([u8; 32], &[u8]), GatewayError> {
        if input.len() >= 32 {
            let (payload_hash, rest) = input.split_at(32);
            let payload_hash = payload_hash.try_into().map_err(|_| InvalidInstruction)?;
            Ok((payload_hash, rest))
        } else {
            Err(InvalidInstruction)
        }
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
    payload_hash: [u8; 32],
) -> Result<Instruction, ProgramError> {
    crate::check_program_account(program_id)?;

    let data = GatewayInstruction::CallContract {
        sender,
        destination_chain: destination_chain.as_bytes(),
        destination_contract_address: destination_contract_address.as_bytes(),
        payload,
        payload_hash,
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
    use solana_sdk::signature::Keypair;
    use solana_sdk::signer::Signer;

    use super::*;

    const TAG: usize = 1;
    const SIZE: usize = std::mem::size_of::<u16>();
    const ID: usize = 50;
    const PAYLOAD: usize = 100;
    const PROOF: usize = 100;
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

    #[test]
    fn round_trip_queue_function() {
        let message_id = rand_array::<3>();
        let proof = rand_array::<1>();
        let payload = rand_array::<2>();

        let instruction = queue(crate::id(), &message_id, &payload, &proof)
            .expect("valid instruction construction");

        let unpacked =
            GatewayInstruction::unpack(&instruction.data).expect("unpacked valid instruction");

        match unpacked {
            GatewayInstruction::Queue {
                id: unpacked_id,
                payload: unpacked_payload,
                proof: unpacked_proof,
            } => {
                assert_eq!(unpacked_id, message_id);
                assert_eq!(unpacked_proof, &proof);
                assert_eq!(unpacked_payload, &payload);
            }
            _ => panic!("Wrong instruction"),
        }
    }

    #[test]
    #[ignore]
    fn unpack_call_contract() {
        todo!()
    }

    #[test]
    fn pack_call_contract() {
        let sender = Keypair::new().pubkey();
        let destination_chain = "ethereum";
        let destination_contract_address = "0x2F43DDFf564Fb260dbD783D55fc6E4c70Be18862";
        let payload = rand_array::<100>();
        let payload_hash = rand_array::<32>();

        let packed = GatewayInstruction::CallContract {
            sender,
            destination_chain: destination_chain.as_bytes(),
            destination_contract_address: destination_contract_address.as_bytes(),
            payload: &payload,
            payload_hash,
        }
        .pack()
        .expect("call contract to be packed");
        let packed = &packed[1..]; // skip tag

        let (packed_sender, rest) = GatewayInstruction::unpack_pubkey(packed).unwrap();
        assert_eq!(packed_sender, sender);
        let mut iterator = SliceIterator::new(rest);
        let packed_destination_chain = next_slice(&mut iterator).unwrap();
        let packed_destination_contract_address = next_slice(&mut iterator).unwrap();
        let packed_payload = next_slice(&mut iterator).unwrap();

        assert_eq!(packed_destination_chain, destination_chain.as_bytes());
        assert_eq!(
            packed_destination_contract_address,
            destination_contract_address.as_bytes()
        );
        assert_eq!(packed_payload, payload);
    }

    #[test]
    fn round_trip_call_contract() {
        let sender = Keypair::new().pubkey();
        let destination_chain = "ethereum";
        let destination_contract_address = "0x2F43DDFf564Fb260dbD783D55fc6E4c70Be18862";
        let payload = rand_array::<100>();
        let payload_hash = rand_array::<32>();

        let instruction = GatewayInstruction::CallContract {
            sender,
            destination_chain: destination_chain.as_bytes(),
            destination_contract_address: destination_contract_address.as_bytes(),
            payload: &payload,
            payload_hash,
        };

        let packed = instruction.pack().expect("call contract to be packed");

        let unpacked = GatewayInstruction::unpack(&packed).expect("call contract to be unpacked");

        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn round_trip_call_contract_function() {
        let sender = Keypair::new().pubkey();
        let destination_chain = "ethereum";
        let destination_contract_address = "0x2F43DDFf564Fb260dbD783D55fc6E4c70Be18862";
        let payload = rand_array::<100>();
        let payload_hash = rand_array::<32>();

        let instruction = call_contract(
            crate::id(),
            sender,
            destination_chain,
            destination_contract_address,
            &payload,
            payload_hash,
        )
        .expect("valid instruction construction");

        let unpacked =
            GatewayInstruction::unpack(&instruction.data).expect("unpacked valid instruction");

        match unpacked {
            GatewayInstruction::CallContract {
                sender: unpacked_sender,
                destination_chain: unpacked_destination_chain,
                destination_contract_address: unpacked_destination_contract_address,
                payload: unpacked_payload,
                payload_hash: unpacked_payload_hash,
            } => {
                assert_eq!(sender, unpacked_sender);
                assert_eq!(destination_chain.as_bytes(), unpacked_destination_chain);
                assert_eq!(
                    destination_contract_address.as_bytes(),
                    unpacked_destination_contract_address
                );
                assert_eq!(payload, unpacked_payload);
                assert_eq!(payload_hash, unpacked_payload_hash);
            }
            _ => panic!("Wrong instruction"),
        };
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
