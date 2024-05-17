//! Module for the `execute_bytes` decoder function.

use borsh::{BorshDeserialize, BorshSerialize};
use itertools::izip;
use solana_program::msg;
use solana_program::pubkey::Pubkey;
use thiserror::Error;

use super::{Proof, SignerSet, U256};
use crate::command::Signature;
use crate::{Address, DestinationProgramId};

/// Decoded command.
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum DecodedCommand {
    ApproveMessages(ApproveMessagesCommand),
    RotateSigners(RotateSignersCommand),
}

impl DecodedCommand {
    pub fn command_id(&self) -> [u8; 32] {
        match self {
            DecodedCommand::ApproveMessages(command) => command.command_id,
            DecodedCommand::RotateSigners(command) => command.command_id,
        }
    }

    pub fn destination_chain(&self) -> u64 {
        match self {
            DecodedCommand::ApproveMessages(command) => command.destination_chain,
            DecodedCommand::RotateSigners(command) => command.destination_chain,
        }
    }

    pub fn destination_program(&self) -> Option<DestinationProgramId> {
        match self {
            DecodedCommand::ApproveMessages(command) => Some(command.destination_program),
            DecodedCommand::RotateSigners(_command) => None,
        }
    }

    pub fn payload_hash(&self) -> Option<[u8; 32]> {
        match self {
            DecodedCommand::ApproveMessages(command) => Some(command.payload_hash),
            DecodedCommand::RotateSigners(_command) => None,
        }
    }
}

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
    #[error("Invalid signer address")]
    /// Invalid signer address
    InvalidSignerAddress,
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

/// Decoded Axelar Message parts.
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ApproveMessagesCommand {
    /// Decoded command id.
    /// It was originally the Axelar Message ID.
    pub command_id: [u8; 32],
    /// Destination chain
    pub destination_chain: u64,
    /// Source chain
    pub source_chain: Vec<u8>,
    /// Source Address
    pub source_address: Vec<u8>,
    /// Destination address
    pub destination_program: DestinationProgramId,
    /// The payload hash
    pub payload_hash: [u8; 32],
}

impl ApproveMessagesCommand {
    fn decode(
        command_id: [u8; 32],
        destination_chain: u64,
        encoded_params: &[u8],
    ) -> Result<Self, DecodeError> {
        type Encoded = (Vec<u8>, Vec<u8>, [u8; 32], Vec<u8>);
        let (source_chain, source_address, destination_program, payload_hash): Encoded =
            bcs::from_bytes(encoded_params).map_err(|_| DecodeError::InvalidCommandParams)?;
        let payload_hash: [u8; 32] = payload_hash
            .try_into()
            .map_err(|_| DecodeError::InvalidPayloadHashSize)?;
        Ok(ApproveMessagesCommand {
            command_id,
            destination_chain,
            source_chain,
            source_address,
            destination_program: DestinationProgramId(Pubkey::from(destination_program)),
            payload_hash,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct RotateSignersCommand {
    /// Decoded command id.
    /// It was originally the Axelar Message ID.
    pub command_id: [u8; 32],
    /// Destination chain
    pub destination_chain: u64,
    pub signer_set: Vec<Address>,
    pub weights: Vec<u128>,
    pub quorum: u128,
}

impl RotateSignersCommand {
    fn decode(
        command_id: [u8; 32],
        destination_chain: u64,
        encoded_params: &[u8],
    ) -> Result<Self, DecodeError> {
        let (signers, weights, quorum) =
            bcs::from_bytes::<(Vec<Vec<u8>>, Vec<u128>, u128)>(encoded_params)
                .map_err(|_| DecodeError::FailedToDecodeProofParts)?;
        let signers = signers
            .into_iter()
            .map(|address| {
                Address::try_from(address.as_slice()).map_err(|_| DecodeError::InvalidSignerAddress)
            })
            .collect::<Result<Vec<Address>, DecodeError>>()?;
        Ok(RotateSignersCommand {
            command_id,
            destination_chain,
            signer_set: signers,
            weights,
            quorum,
        })
    }
}

/// New definition of the `multisig_prover::types::CommandType` enum just so I
/// can rederive the `Borsh` traits.
#[cosmwasm_schema::cw_serde]
#[derive(Eq, BorshSerialize, BorshDeserialize)]
pub enum CommandType {
    ApproveMessages,
    TransferOperatorship,
}

impl DecodedCommand {
    fn decode(
        command_id: [u8; 32],
        type_: &str,
        destination_chain_id: u64,
        encoded_params: &[u8],
    ) -> Result<Self, DecodeError> {
        let type_ = decode_command_type(type_)?;
        match type_ {
            CommandType::ApproveMessages => {
                let message = ApproveMessagesCommand::decode(
                    command_id,
                    destination_chain_id,
                    encoded_params,
                )?;
                Ok(DecodedCommand::ApproveMessages(message))
            }
            CommandType::TransferOperatorship => {
                let message =
                    RotateSignersCommand::decode(command_id, destination_chain_id, encoded_params)?;
                Ok(DecodedCommand::RotateSigners(message))
            }
        }
    }
}

#[inline]
fn decode_command_type(encoded_type: &str) -> Result<CommandType, DecodeError> {
    match encoded_type {
        "approveContractCall" => Ok(CommandType::ApproveMessages),
        "transferOperatorship" => Ok(CommandType::TransferOperatorship),
        _ => Err(DecodeError::InvalidCommandType),
    }
}

/// Decoded command batch.
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct DecodedCommandBatch {
    /// The decoded commands.
    pub commands: Vec<DecodedCommand>,
}

#[inline]
fn build_proof_from_raw_parts(
    addresses: Vec<Vec<u8>>,
    weights: Vec<u128>,
    quorum: u128,
    signatures: Vec<Vec<u8>>,
) -> Result<Proof, DecodeError> {
    let signers = {
        let addresses = addresses
            .into_iter()
            .map(|address| Address::try_from(address.as_slice()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|address_error| {
                msg!("Invalid signer address: {}", address_error);
                DecodeError::InvalidSignerAddress
            })?;

        let weights: Vec<U256> = weights.into_iter().map(Into::into).collect();
        SignerSet::new(addresses, weights, quorum.into())
    };
    let signatures = signatures
        .into_iter()
        .map(TryInto::<Signature>::try_into)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| DecodeError::InvalidSignature)?;

    Ok(Proof::new(signers, signatures))
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
fn decode_command_batch(
    command_batch_bytes: &[u8],
) -> Result<(DecodedCommandBatch, [u8; 32]), DecodeError> {
    let hash = axelar_bcs_encoding::msg_digest(command_batch_bytes)?;
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

    Ok((DecodedCommandBatch { commands }, hash))
}

/// Decodes the `execute_data` bytes into a [`Proof`] and
/// [`DecodedCommandBatch`] tuple.
pub fn decode(bytes: &[u8]) -> Result<(Proof, DecodedCommandBatch, [u8; 32]), DecodeError> {
    // Split into:
    // 1. command batch parts
    // 2. proof parts
    let (command_batch_bytes, proof_bytes): (Vec<u8>, Vec<u8>) =
        bcs::from_bytes(bytes).map_err(|_| DecodeError::FailedToSplitExecuteData)?;
    let proof = decode_proof(&proof_bytes)?;
    let (commands, hash) = decode_command_batch(&command_batch_bytes)?;
    Ok((proof, commands, hash))
}

mod axelar_bcs_encoding {

    use super::*;

    pub fn msg_digest(data: &[u8]) -> Result<[u8; 32], DecodeError> {
        // TODO use solana_program::hash::keccak here
        use sha3::{Digest, Keccak256};
        // Sui is just mimicking EVM here
        let unsigned = [
            "\x19Sui Signed Message:\n".as_bytes(), // Keccek256 hash length = 32
            data,
        ]
        .concat();

        Ok(Keccak256::digest(unsigned).into())
    }
}

#[test]
fn decode_execute_data_arbitrary() -> anyhow::Result<()> {
    let execute_data = hex::decode("b00139050000000000000154a54429224fc8602f5d93073bf4ce4593e89491b09eabd1618727445675757e0113617070726f7665436f6e747261637443616c6c0170036574682a30786132644438313763326644633361323939366631413531373443463866314161454434363645383202a212de6a9dfa3a69e22387acfbafbb1a9e591bd9d636e7895dcfc8de05f331203e50a012285f8e7ec59b558179cd546c55c477ebe16202aac7d7747e25be03be8701012102bf43acfda9d3c7b9007c4b0420fc77c582975376446b43500fdf25710206488d0101000000000000000000000000000000010000000000000000000000000000000141d68dd2ed49aa1edaa1286bb6c09bb8611b9fb6af0c18c7823f4fccd7ccc91f7d1d41ccba1d45e9e9063d832db681978a364d6ea7b44592badf82d0b1281465b801")?;
    decode(&execute_data)?;
    Ok(())
}

#[test]
fn decode_transfer_operatorship() {
    // The hex is extracted from this testcase: https://github.com/axelarnetwork/axelar-amplifier/blob/e04a85fc635448559cdd265da582d1f49a6b8f52/contracts/multisig-prover/src/encoding/bcs.rs#L224C8-L224C41
    let transfer_message = hex::decode("05210274b5d2a4c55d7edbbf9cc210c4d25adbb6194d6b444816235c82984bee51825521028584592624e742ba154c02df4c0b06e4e8a957ba081083ea9fe5309492aa6c7b2102a670f57de55b8b39b4cb051e178ca8fb3fe3a78cdde7f8238baf5e6ce18931852103c6ddb0fcee7b528da1ef3c9eed8d51eeacd7cc28a8baa25c33037c5562faa6e42103d123ce370b163acd576be0e32e436bb7e63262769881d35fa3573943bf6c6f81050a0000000000000000000000000000000a0000000000000000000000000000000a0000000000000000000000000000000a0000000000000000000000000000000a0000000000000000000000000000001e000000000000000000000000000000").unwrap();
    let destination_chain = 1;
    let command_id = [0; 32];

    // we just want to see if it can be decoded
    let _transfer_message =
        RotateSignersCommand::decode(command_id, destination_chain, transfer_message.as_ref())
            .unwrap();
}

#[test]
fn decode_execute_data_from_axelar_repo() -> anyhow::Result<()> {
    // Copied from https://github.com/axelarnetwork/axelar-amplifier/blob/e04a85fc635448559cdd265da582d1f49a6b8f52/contracts/multisig-prover/src/encoding/bcs.rs#L509
    let approval = hex::decode("8a02010000000000000002000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020213617070726f7665436f6e747261637443616c6c13617070726f7665436f6e747261637443616c6c0249034554480330783000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000004c064158454c415203307831000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000087010121037286a4f1177bea06c8e15cf6ec3df0b7747a01ac2329ca2999dfd74eff59902801640000000000000000000000000000000a0000000000000000000000000000000141ef5ce016a4beed7e11761e5831805e962fca3d8901696a61a6ffd3af2b646bdc3740f64643bdb164b8151d1424eb4943d03f71e71816c00726e2d68ee55600c600")?;

    let (proof, command_batch, command_batch_hash) = decode(&approval)?;

    // Check proof
    let signer_pubkey: Address =
        hex::decode("037286a4f1177bea06c8e15cf6ec3df0b7747a01ac2329ca2999dfd74eff599028")?
            .as_slice()
            .try_into()?;
    let signature: Signature = hex::decode("ef5ce016a4beed7e11761e5831805e962fca3d8901696a61a6ffd3af2b646bdc3740f64643bdb164b8151d1424eb4943d03f71e71816c00726e2d68ee55600c600")?.try_into()?;
    assert_eq!(proof.signer_set.addresses(), &[signer_pubkey]);
    assert_eq!(proof.signer_set.threshold(), &10u128.into());
    assert_eq!(proof.signer_set.weights(), &[100u128.into()]);
    assert_eq!(proof.signatures(), &[signature]);

    // Check command batch
    let mut message_id1 = [0u8; 32];
    message_id1[31] = 1;
    let mut message_id2 = [0u8; 32];
    message_id2[31] = 2;
    let zero_array = [0u8; 32];
    let command1 = DecodedCommand::ApproveMessages(ApproveMessagesCommand {
        command_id: message_id1,
        destination_chain: 1,
        source_chain: b"ETH".to_vec(),
        source_address: b"0x0".to_vec(),
        destination_program: DestinationProgramId(Pubkey::from(zero_array)),
        payload_hash: zero_array,
    });
    let command2 = DecodedCommand::ApproveMessages(ApproveMessagesCommand {
        command_id: message_id2,
        destination_chain: 1,
        source_chain: b"AXELAR".to_vec(),
        source_address: b"0x1".to_vec(),
        destination_program: DestinationProgramId(Pubkey::from(zero_array)),
        payload_hash: zero_array,
    });
    let expected = DecodedCommandBatch {
        commands: vec![command1, command2],
    };
    let expected_hash: [u8; 32] =
        hex::decode("bdaa19807472e2261f2ace59a1c368fd45ee858dad21c1612dae2553052f6fdf")?
            .try_into()
            .expect("vector with 32 elements");

    assert_eq!(command_batch, expected);
    assert_eq!(command_batch_hash, expected_hash);
    Ok(())
}

#[test]
fn decode_custom_execute_data() -> anyhow::Result<()> {
    let execute_data = test_fixtures::execute_data::create_execute_data(5, 3, 2)?;

    let (proof, command_batch, command_batch_hash) = decode(&execute_data)?;

    assert_eq!(command_batch.commands.len(), 5);
    assert_eq!(proof.signer_set.addresses().len(), 3);
    assert_eq!(proof.signatures().len(), 3);
    assert_eq!(*proof.signer_set.threshold(), 2u8.into());

    if let Err(error) = proof.validate_signatures(&command_batch_hash) {
        panic!("Invalid proof: {error}")
    };

    Ok(())
}
