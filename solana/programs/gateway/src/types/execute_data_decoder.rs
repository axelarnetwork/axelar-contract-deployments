//! Module for the `execute_bytes` decoder function.

use std::borrow::Cow;

use axelar_message_primitives::{
    AxelarMessageParams, CommandId, DataPayloadHash, DestinationProgramId, SourceAddress,
    SourceChain,
};
use itertools::izip;
use multisig_prover::encoding::Data;
use multisig_prover::types::{Command, CommandType};
use solana_program::msg;
use solana_program::pubkey::Pubkey;
use thiserror::Error;

use crate::error::GatewayError;
use crate::types::address::Address;
use crate::types::operator::Operators;
use crate::types::proof::Proof;
use crate::types::signature::Signature;
use crate::types::u256::U256;

type DecodedCommandBatchParts = (u64, Vec<[u8; 32]>, Vec<String>, Vec<Vec<u8>>);
/// Addresses, Weights, Quorum, and Signatures
type DecodedProofParts = (Vec<Vec<u8>>, Vec<u128>, u128, Vec<Vec<u8>>);

#[derive(Error, Debug)]
/// Errors that might happen when decoding `execute_bytes`.
pub enum DecodeError {
    #[error("Invalid command params")]
    /// Invalid command params
    InvalidCommandParams,
    #[error("Expected a payload hash with 32 bytes")]
    /// Expected a payload hash with 32 bytes
    InvalidPayloadHashSize,
    #[error("Invalid command type")]
    /// Invalid command type
    InvalidCommandType,
    #[error("Invalid operator address")]
    /// Invalid operator address
    InvalidOperatorAddress,
    #[error("Invalid signature")]
    /// Invalid signature
    InvalidSignature,
    #[error("Falied to split `execute_data` into command batch and proof")]
    /// Falied to split `execute_data` into command batch and proof
    FailedToSplitExecuteData,
    #[error("Falied to decode proof parts")]
    /// Falied to decode proof parts
    FailedToDecodeProofParts,
    #[error("Falied to decode command batch parts")]
    /// Falied to decode command batch parts
    FailedToDecodeCommandBatchParts,
    #[error("Falied to reencode command batch data")]
    ///Falied to reencode command batch data
    FailedToReencodeCommandBatchData,
}

impl From<DecodeError> for GatewayError {
    /// Logs the `[DecodeError]` before turning into an opaque [`GatewayError`].
    fn from(error: DecodeError) -> Self {
        msg!("Error decoding `execute_data`: {}", error);
        GatewayError::FailedToDecodeExecuteData
    }
}

/// Decoded Axelar Message parts.
#[derive(Debug, PartialEq)]
pub struct DecodedMessage {
    /// Decoded command id.
    /// It was originally the Axelar Message ID.
    pub id: [u8; 32],
    /// Destination chain
    pub destination_chain: u64,
    /// Source chain
    pub source_chain: String,
    /// Source Address
    pub source_address: String,
    /// Destination address
    pub destination_address: [u8; 32],
    /// The payload hash
    pub payload_hash: [u8; 32],
}

impl DecodedMessage {
    fn decode(
        command_id: [u8; 32],
        destination_chain: u64,
        encoded_params: &[u8],
    ) -> Result<Self, DecodeError> {
        type Encoded = (String, String, [u8; 32], Vec<u8>);
        let (source_chain, source_address, destination_address, payload_hash): Encoded =
            bcs::from_bytes(encoded_params).map_err(|_| DecodeError::InvalidCommandParams)?;
        let payload_hash: [u8; 32] = payload_hash
            .try_into()
            .map_err(|_| GatewayError::MalformedProof)
            .map_err(|_| DecodeError::InvalidPayloadHashSize)?;
        Ok(DecodedMessage {
            id: command_id,
            destination_chain,
            source_chain,
            source_address,
            destination_address,
            payload_hash,
        })
    }
}

/// Decoded command.
#[derive(Debug, PartialEq)]
pub struct DecodedCommand {
    /// The decoded command type
    pub type_: CommandType,
    /// The decoded Axelar Message
    pub message: DecodedMessage,
}

impl DecodedCommand {
    fn decode(
        command_id: [u8; 32],
        type_: &str,
        destination_chain_id: u64,
        encoded_params: &[u8],
    ) -> Result<Self, DecodeError> {
        let type_ = decode_command_type(type_)?;
        let message = DecodedMessage::decode(command_id, destination_chain_id, encoded_params)?;
        Ok(DecodedCommand { type_, message })
    }
}

impl<'a> From<&'a DecodedCommand> for AxelarMessageParams<'a> {
    fn from(command: &'a DecodedCommand) -> Self {
        let DecodedMessage {
            id,
            source_chain,
            source_address,
            payload_hash,
            destination_address,
            ..
        } = &command.message;

        let message_id = CommandId(Cow::Borrowed(id));
        let source_chain = SourceChain(Cow::Borrowed(source_chain));
        let source_address = SourceAddress(source_address.as_bytes());
        let destination_pubkey = DestinationProgramId(Pubkey::from(*destination_address));
        let payload_hash = DataPayloadHash(Cow::Borrowed(payload_hash));

        AxelarMessageParams {
            command_id: message_id,
            source_chain,
            source_address,
            destination_program: destination_pubkey,
            payload_hash,
        }
    }
}

#[inline]
fn decode_command_type(encoded_type: &str) -> Result<CommandType, DecodeError> {
    match encoded_type {
        "approveContractCall" => Ok(CommandType::ApproveContractCall),
        "transferOperatorship" => Ok(CommandType::TransferOperatorship),
        _ => Err(DecodeError::InvalidCommandType),
    }
}

/// Decoded command batch.
#[derive(Debug, PartialEq)]
pub struct DecodedCommandBatch {
    /// The decoded commands.
    pub commands: Vec<DecodedCommand>,
    /// The hash of the bytes used to decode this command batch.
    pub hash: [u8; 32],
}

#[inline]
fn build_proof_from_raw_parts(
    addresses: Vec<Vec<u8>>,
    weights: Vec<u128>,
    quorum: u128,
    signatures: Vec<Vec<u8>>,
) -> Result<Proof, DecodeError> {
    let operators = {
        let addresses = addresses
            .into_iter()
            .map(|address| Address::try_from(address.as_slice()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|address_error| {
                msg!("Invalid operator address: {}", address_error);
                DecodeError::InvalidOperatorAddress
            })?;

        let weights: Vec<U256> = weights.into_iter().map(Into::into).collect();
        Operators::new(addresses, weights, quorum.into())
    };
    let signatures = signatures
        .into_iter()
        .map(TryInto::<Signature>::try_into)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| DecodeError::InvalidSignature)?;

    Ok(Proof::new(operators, signatures))
}

#[inline]
fn decode_proof(proof_bytes: &[u8]) -> Result<Proof, DecodeError> {
    let (addresses, weights, quorum, signatures): DecodedProofParts =
        bcs::from_bytes(proof_bytes).map_err(|_| DecodeError::FailedToDecodeProofParts)?;
    build_proof_from_raw_parts(addresses, weights, quorum, signatures)
}

/* TODO:

This function should be more efficent.

Currently, it recombine the CommandBatch internal values twice to produce:
1) a `DecodedCommandBatch` value, which is part of the original return value, and
2) a `Data` value, which is used to recreate the CommandBatch hash, which is the cryptographic
    message that was signed over.

Those types are very much alike, and returning just the `Data` type would suffice.
*/
#[inline]
fn decode_command_batch(command_batch_bytes: &[u8]) -> Result<DecodedCommandBatch, DecodeError> {
    // Decode command batch parts
    let (destination_chain_id, commands_ids, commands_types, commands_params): DecodedCommandBatchParts =
                bcs::from_bytes(command_batch_bytes).map_err(|_| DecodeError::FailedToDecodeCommandBatchParts)?;

    // Assert parts are aligned
    if commands_ids.len() != commands_types.len() && commands_types.len() != commands_params.len() {
        return Err(DecodeError::FailedToDecodeCommandBatchParts);
    }

    // Build command batch from raw parts
    let commands = izip!(&commands_ids, &commands_types, &commands_params)
        .map(|(id, type_, encoded_params)| {
            DecodedCommand::decode(*id, type_, destination_chain_id, encoded_params)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // TODO: repurpose this value as part of the returned type.
    let data: Data = reconstruct_command_batch_data(
        destination_chain_id,
        commands_ids,
        commands_types,
        commands_params,
    )?;

    let hash = axelar_bcs_encoding::msg_digest(&data)?;

    Ok(DecodedCommandBatch { commands, hash })
}

fn reconstruct_command_batch_data(
    destination_chain_id: u64,
    commands_ids: Vec<[u8; 32]>,
    commands_types: Vec<String>,
    commands_params: Vec<Vec<u8>>,
) -> Result<Data, DecodeError> {
    let commands: Vec<Command> = izip!(
        commands_ids.into_iter(),
        commands_types.into_iter(),
        commands_params.into_iter()
    )
    .map(|(id, ty, params)| -> Result<Command, DecodeError> {
        Ok(Command {
            id: id.into(),
            ty: decode_command_type(&ty)?,
            params: params.into(),
        })
    })
    .collect::<Result<Vec<_>, _>>()?;

    Ok(Data {
        destination_chain_id: destination_chain_id.into(),
        commands,
    })
}

/// Decodes the `execute_data` bytes into a [`Proof`] and
/// [`DecodedCommandBatch`] tuple.
pub fn decode(bytes: &[u8]) -> Result<(Proof, DecodedCommandBatch), DecodeError> {
    // Split into:
    // 1. command batch parts
    // 2. proof parts
    let (command_batch_bytes, proof_bytes): (Vec<u8>, Vec<u8>) =
        bcs::from_bytes(bytes).map_err(|_| DecodeError::FailedToSplitExecuteData)?;
    let proof = decode_proof(&proof_bytes)?;
    let commands = decode_command_batch(&command_batch_bytes)?;
    Ok((proof, commands))
}

mod axelar_bcs_encoding {
    use std::mem::size_of;

    use super::*;

    pub fn msg_digest(data: &Data) -> Result<[u8; 32], DecodeError> {
        use sha3::{Digest, Keccak256};
        // Sui is just mimicking EVM here
        let unsigned = [
            "\x19Sui Signed Message:\n".as_bytes(), // Keccek256 hash length = 32
            encode(data)?.as_slice(),
        ]
        .concat();

        Ok(Keccak256::digest(unsigned).into())
    }

    pub fn encode(data: &Data) -> Result<Vec<u8>, DecodeError> {
        // destination chain id must be u64 for sui
        let destination_chain_id = u256_to_u64(data.destination_chain_id)?;

        let num_commands = data.commands.len();
        let mut command_ids = Vec::with_capacity(num_commands);
        let mut command_types = Vec::with_capacity(num_commands);
        let mut command_params = Vec::with_capacity(num_commands);
        for command in &data.commands {
            command_ids.push(make_command_id(command.id.to_vec())?);
            command_types.push(command.ty.to_string());
            command_params.push(command.params.to_vec());
        }

        bcs::to_bytes(&(
            destination_chain_id,
            command_ids,
            command_types,
            command_params,
        ))
        .map_err(|_bcs_error| DecodeError::FailedToReencodeCommandBatchData)
    }

    #[inline]
    fn u256_to_u64(number: cosmwasm_std::Uint256) -> Result<u64, DecodeError> {
        let u256_bytes_le = number.to_le_bytes();
        // check if it would fit into an u64
        if u256_bytes_le[size_of::<u64>()..].iter().any(|&x| x != 0) {
            return Err(DecodeError::FailedToReencodeCommandBatchData);
        }
        let mut u64_arr = [0u8; size_of::<u64>()];
        u64_arr.copy_from_slice(&u256_bytes_le[0..size_of::<u64>()]);

        Ok(u64::from_le_bytes(u64_arr))
    }

    #[inline]
    fn make_command_id(command_id: Vec<u8>) -> Result<[u8; 32], DecodeError> {
        // command-ids are fixed length sequences
        command_id.try_into().map_err(|_| {
            msg!("Decode error: decoded command_id dosen't have 32 bytes");
            DecodeError::FailedToReencodeCommandBatchData
        })
    }
}

#[test]
fn decode_execute_data_arbitrary() -> anyhow::Result<()> {
    let execute_data = hex::decode("b00139050000000000000154a54429224fc8602f5d93073bf4ce4593e89491b09eabd1618727445675757e0113617070726f7665436f6e747261637443616c6c0170036574682a30786132644438313763326644633361323939366631413531373443463866314161454434363645383202a212de6a9dfa3a69e22387acfbafbb1a9e591bd9d636e7895dcfc8de05f331203e50a012285f8e7ec59b558179cd546c55c477ebe16202aac7d7747e25be03be8701012102bf43acfda9d3c7b9007c4b0420fc77c582975376446b43500fdf25710206488d0101000000000000000000000000000000010000000000000000000000000000000141d68dd2ed49aa1edaa1286bb6c09bb8611b9fb6af0c18c7823f4fccd7ccc91f7d1d41ccba1d45e9e9063d832db681978a364d6ea7b44592badf82d0b1281465b801")?;
    decode(&execute_data)?;
    Ok(())
}

#[test]
fn decode_execute_data_from_axelar_repo() -> anyhow::Result<()> {
    // Copied from https://github.com/axelarnetwork/axelar-amplifier/blob/e04a85fc635448559cdd265da582d1f49a6b8f52/contracts/multisig-prover/src/encoding/bcs.rs#L509
    let approval = hex::decode("8a02010000000000000002000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020213617070726f7665436f6e747261637443616c6c13617070726f7665436f6e747261637443616c6c0249034554480330783000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000004c064158454c415203307831000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000087010121037286a4f1177bea06c8e15cf6ec3df0b7747a01ac2329ca2999dfd74eff59902801640000000000000000000000000000000a0000000000000000000000000000000141ef5ce016a4beed7e11761e5831805e962fca3d8901696a61a6ffd3af2b646bdc3740f64643bdb164b8151d1424eb4943d03f71e71816c00726e2d68ee55600c600")?;

    let (proof, command_batch) = decode(&approval)?;

    // Check proof
    let signer_pubkey: Address =
        hex::decode("037286a4f1177bea06c8e15cf6ec3df0b7747a01ac2329ca2999dfd74eff599028")?
            .as_slice()
            .try_into()?;
    let signature: Signature = hex::decode("ef5ce016a4beed7e11761e5831805e962fca3d8901696a61a6ffd3af2b646bdc3740f64643bdb164b8151d1424eb4943d03f71e71816c00726e2d68ee55600c600")?.try_into()?;
    assert_eq!(proof.operators.addresses(), &[signer_pubkey]);
    assert_eq!(proof.operators.threshold(), &10u128.into());
    assert_eq!(proof.operators.weights(), &[100u128.into()]);
    assert_eq!(proof.signatures(), &[signature]);

    // Check command batch
    let mut message_id1 = [0u8; 32];
    message_id1[31] = 1;
    let mut message_id2 = [0u8; 32];
    message_id2[31] = 2;
    let zero_array = [0u8; 32];
    let command1 = DecodedCommand {
        type_: CommandType::ApproveContractCall,
        message: DecodedMessage {
            id: message_id1,
            destination_chain: 1,
            source_chain: "ETH".to_string(),
            source_address: "0x0".to_string(),
            destination_address: zero_array,
            payload_hash: zero_array,
        },
    };
    let command2 = DecodedCommand {
        type_: CommandType::ApproveContractCall,
        message: DecodedMessage {
            id: message_id2,
            destination_chain: 1,
            source_chain: "AXELAR".to_string(),
            source_address: "0x1".to_string(),
            destination_address: zero_array,
            payload_hash: zero_array,
        },
    };
    let expected = DecodedCommandBatch {
        commands: vec![command1, command2],
        hash: hex::decode("bdaa19807472e2261f2ace59a1c368fd45ee858dad21c1612dae2553052f6fdf")?
            .try_into()
            .expect("vector with 32 elements"),
    };

    assert_eq!(command_batch, expected);
    Ok(())
}

#[test]
fn decode_custom_execute_data() -> anyhow::Result<()> {
    let execute_data = test_fixtures::execute_data::create_execute_data(5, 3, 2)?;

    let (proof, command_batch) = decode(&execute_data)?;

    assert_eq!(command_batch.commands.len(), 5);
    assert_eq!(proof.operators.addresses().len(), 3);
    assert_eq!(proof.signatures().len(), 3);
    assert_eq!(*proof.operators.threshold(), 2u8.into());

    if let Err(error) = proof.validate(&command_batch.hash) {
        panic!("Invalid proof: {error}")
    };

    Ok(())
}
