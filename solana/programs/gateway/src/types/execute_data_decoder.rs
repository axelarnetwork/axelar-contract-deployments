//! Module for the `execute_bytes` decoder function.

use auth_weighted::types::address::Address;
use auth_weighted::types::operator::Operators;
use auth_weighted::types::proof::Proof;
use auth_weighted::types::signature::Signature;
use auth_weighted::types::u256::U256;
use itertools::izip;
use multisig_prover::types::CommandType;
use solana_program::msg;
use thiserror::Error;

use crate::error::GatewayError;

type DecodedCommandBatchParts = (u64, Vec<[u8; 32]>, Vec<String>, Vec<Vec<u8>>);
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
    id: [u8; 32],
    destination_chain: u64,
    source_chain: String,
    source_address: String,
    destination_address: [u8; 32],
    payload_hash: [u8; 32],
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
    type_: CommandType,
    message: DecodedMessage,
}

impl DecodedCommand {
    fn decode(
        command_id: [u8; 32],
        type_: &str,
        destination_chain_id: u64,
        encoded_params: &[u8],
    ) -> Result<Self, DecodeError> {
        let type_ = match type_ {
            "approveContractCall" => CommandType::ApproveContractCall,
            "transferOperatorship" => CommandType::TransferOperatorship,
            _ => return Err(DecodeError::InvalidCommandType),
        };
        let message = DecodedMessage::decode(command_id, destination_chain_id, encoded_params)?;
        Ok(DecodedCommand { type_, message })
    }
}

fn build_proof_from_raw_parts(
    addresses: Vec<Vec<u8>>,
    weights: Vec<u128>,
    quorum: u128,
    signatures: Vec<Vec<u8>>,
) -> Result<Proof, DecodeError> {
    let operators = {
        let addresses = addresses
            .into_iter()
            .map(TryInto::<Address>::try_into)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| DecodeError::InvalidOperatorAddress)?;

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

/// Decodes the `execute_data` bytes into a [`Proof`] and
/// [`Vec<DecodedCommand>`] tuple.
pub fn decode(bytes: &[u8]) -> Result<(Proof, Vec<DecodedCommand>), DecodeError> {
    // Split into:
    // 1. command batch parts
    // 2. proof parts
    let (command_batch_bytes, proof_bytes): (Vec<u8>, Vec<u8>) =
        bcs::from_bytes(bytes).map_err(|_| DecodeError::FailedToSplitExecuteData)?;

    let proof = {
        // Decode proof parts
        let (addresses, weights, quorum, signatures): DecodedProofParts =
            bcs::from_bytes(&proof_bytes).map_err(|_| DecodeError::FailedToDecodeProofParts)?;
        build_proof_from_raw_parts(addresses, weights, quorum, signatures)?
    };

    let commands = {
        // Decode command batch parts
        let (destination_chain_id, commands_ids, commands_types, commands_params): DecodedCommandBatchParts =
                bcs::from_bytes(&command_batch_bytes).map_err(|_| DecodeError::FailedToDecodeCommandBatchParts)?;
        // Build command batch from raw parts
        izip!(&commands_ids, &commands_types, &commands_params)
            .map(|(id, type_, encoded_params)| {
                DecodedCommand::decode(*id, type_, destination_chain_id, encoded_params)
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    Ok((proof, commands))
}

#[test]
fn decode_execute_data() -> anyhow::Result<()> {
    // Copied from https://github.com/axelarnetwork/axelar-amplifier/blob/e04a85fc635448559cdd265da582d1f49a6b8f52/contracts/multisig-prover/src/encoding/bcs.rs#L509
    let approval = hex::decode("8a02010000000000000002000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020213617070726f7665436f6e747261637443616c6c13617070726f7665436f6e747261637443616c6c0249034554480330783000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000004c064158454c415203307831000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000087010121037286a4f1177bea06c8e15cf6ec3df0b7747a01ac2329ca2999dfd74eff59902801640000000000000000000000000000000a0000000000000000000000000000000141ef5ce016a4beed7e11761e5831805e962fca3d8901696a61a6ffd3af2b646bdc3740f64643bdb164b8151d1424eb4943d03f71e71816c00726e2d68ee55600c600")?;

    let (proof, command_batch) = decode(&approval)?;

    // Check proof
    let signer_pubkey: Address =
        hex::decode("037286a4f1177bea06c8e15cf6ec3df0b7747a01ac2329ca2999dfd74eff599028")?
            .try_into()?;
    let signature: Signature = hex::decode("ef5ce016a4beed7e11761e5831805e962fca3d8901696a61a6ffd3af2b646bdc3740f64643bdb164b8151d1424eb4943d03f71e71816c00726e2d68ee55600c6")?.try_into()?;
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
    assert_eq!(command_batch, &[command1, command2]);
    Ok(())
}
