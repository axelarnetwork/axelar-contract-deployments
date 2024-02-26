use anyhow::{anyhow, ensure, Result};
use connection_router::Message as AxelarMessage;
use cosmwasm_std::{Addr, Uint256};
use libsecp256k1::{sign, Message, PublicKey, SecretKey};
use multisig::key::{KeyType, PublicKey as AxelarPublicKey, Signature};
use multisig::msg::Signer;
use multisig_prover::encoding::{CommandBatchBuilder, Encoder};
use multisig_prover::types::CommandBatch;

use crate::axelar_message::message;

#[derive(Debug, Clone)]
pub struct TestSigner {
    pub weight: Uint256,
    pub secret_key: SecretKey,
    pub public_key: AxelarPublicKey,
}

impl TestSigner {
    pub fn uncompressed_public_key(&self) -> [u8; 64] {
        let public_key = PublicKey::from_secret_key(&self.secret_key);
        let serialized = public_key.serialize();
        let (_tag, public_key_bytes) = serialized.split_at(1);
        public_key_bytes.try_into().unwrap()
    }
}

impl From<TestSigner> for Signer {
    fn from(val: TestSigner) -> Self {
        let TestSigner {
            weight, public_key, ..
        } = val;

        // Signer address is not used to encode the Proof.
        let address = Addr::unchecked("");

        Signer {
            address,
            weight,
            pub_key: public_key,
        }
    }
}

pub struct Fixtures {
    pub messages: Vec<AxelarMessage>,
    pub signers: Vec<TestSigner>,
    pub command_batch: CommandBatch,
    pub signatures: Vec<Option<Signature>>,
}

fn fixtures(num_messages: usize, num_signers: usize) -> Result<Fixtures> {
    let messages: Vec<AxelarMessage> = (0..num_messages)
        .map(|_| message())
        .collect::<Result<_, _>>()?;
    let signers: Vec<TestSigner> = (0..num_signers)
        .map(|_| create_signer())
        .collect::<Result<_, _>>()?;
    let command_batch = create_command_batch(&messages)?;
    let signatures: Vec<Option<Signature>> = sign_batch(&command_batch, &signers)?;
    Ok(Fixtures {
        messages,
        signers,
        command_batch,
        signatures,
    })
}

pub fn create_execute_data(
    num_messages: usize,
    num_signers: usize,
    quorum: u128,
) -> Result<Vec<u8>> {
    let Fixtures {
        signers,
        command_batch,
        signatures,
        ..
    } = fixtures(num_messages, num_signers)?;
    encode(&command_batch, signers, signatures, quorum)
}

pub fn create_command_batch(messages: &[AxelarMessage]) -> Result<CommandBatch> {
    let mut builder = CommandBatchBuilder::new(555u64.into(), Encoder::Bcs);
    for msg in messages {
        builder.add_message(msg.clone())?;
    }
    Ok(builder.build()?)
}

pub fn create_signer() -> Result<TestSigner> {
    create_signer_with_weight(1)
}

pub fn create_signer_with_weight(weight: u128) -> Result<TestSigner> {
    let secret_key = SecretKey::random(&mut rand_core::OsRng);
    let public_key = PublicKey::from_secret_key(&secret_key);
    let public_key_bytes = public_key.serialize_compressed();
    let public_key: AxelarPublicKey = (KeyType::Ecdsa, public_key_bytes.into()).try_into()?;

    Ok(TestSigner {
        secret_key,
        public_key,
        weight: cosmwasm_std::Uint256::from_u128(weight),
    })
}

pub fn sign_batch(
    command_batch: &CommandBatch,
    signers: &[TestSigner],
) -> Result<Vec<Option<Signature>>> {
    let mut signatures = vec![];
    let message_to_sign: [u8; 32] = command_batch.msg_digest().as_ref().try_into()?;
    let message = Message::parse(&message_to_sign);

    for signer in signers {
        // Sign the message
        let (signature, recovery_id) = sign(&message, &signer.secret_key);

        // Concatenate signature and recovery byte
        let mut extended_signature = [0u8; 65];
        extended_signature[0..64].copy_from_slice(&signature.serialize());
        extended_signature[64] = recovery_id.serialize();

        // Convert into the Axelar signature type
        let axelar_sig: multisig::key::Signature =
            (KeyType::Ecdsa, extended_signature.into()).try_into()?;
        assert!(matches!(axelar_sig, Signature::EcdsaRecoverable(_))); // confidence check
        signatures.push(Some(axelar_sig));
    }
    Ok(signatures)
}

pub fn encode(
    command_batch: &CommandBatch,
    signers: Vec<TestSigner>,
    signatures: Vec<Option<Signature>>,
    quorum: u128,
) -> Result<Vec<u8>> {
    ensure!(
        signers.len() == signatures.len(),
        "signers and signature missmatch"
    );
    let quorum: Uint256 = quorum.into();
    let signers_and_signatures: Vec<(Signer, Option<Signature>)> = signers
        .into_iter()
        .map(Into::into)
        .zip(signatures)
        .collect();

    axelar_bcs_encoding::encode_execute_data(command_batch, quorum, signers_and_signatures)
        .map_err(|e| anyhow!("failed to encode execute_data: {e}"))
        .map(|hexbinary| hexbinary.to_vec())
}

/// Code copied from https://github.com/axelarnetwork/axelar-amplifier/blob/d34dce84e7c16327f05de1fea659fe094306bb7f/contracts/multisig-prover/src/encoding/bcs.rs
mod axelar_bcs_encoding {
    use std::convert::identity;

    use cosmwasm_std::{HexBinary, Uint256};
    use itertools::Itertools;
    use multisig::key::Signature;
    use multisig::msg::Signer;
    use multisig_prover::encoding::Data;
    use multisig_prover::error::ContractError;
    use multisig_prover::types::{CommandBatch, Operator};

    pub fn encode_execute_data(
        command_batch: &CommandBatch,
        quorum: Uint256,
        signers: Vec<(Signer, Option<Signature>)>,
    ) -> Result<HexBinary, ContractError> {
        let signers = signers
            .into_iter()
            .map(|(signer, signature)| {
                let mut signature = signature;
                if let Some(Signature::Ecdsa(nonrecoverable)) = signature {
                    signature = nonrecoverable
                        .to_recoverable(
                            command_batch.msg_digest().as_slice(),
                            &signer.pub_key,
                            identity,
                        )
                        .map(Signature::EcdsaRecoverable)
                        .ok();
                }
                assert!(signature.is_some(), "Signature was erased");
                (signer, signature)
            })
            .collect::<Vec<_>>();
        let input = bcs::to_bytes(&(
            encode(&command_batch.data).to_vec(),
            encode_proof(quorum, signers)?.to_vec(),
        ))?;
        Ok(input.into())
    }

    fn encode(data: &Data) -> HexBinary {
        // destination chain id must be u64 for sui
        let destination_chain_id = u256_to_u64(data.destination_chain_id);

        let (commands_ids, command_types, command_params): (
            Vec<[u8; 32]>,
            Vec<String>,
            Vec<Vec<u8>>,
        ) = data
            .commands
            .iter()
            .map(|command| {
                (
                    make_command_id(&command.id),
                    command.ty.to_string(),
                    command.params.to_vec(),
                )
            })
            .multiunzip();

        bcs::to_bytes(&(
            destination_chain_id,
            commands_ids,
            command_types,
            command_params,
        ))
        .expect("couldn't encode batch as bcs")
        .into()
    }

    fn encode_proof(
        quorum: Uint256,
        signers: Vec<(Signer, Option<Signature>)>,
    ) -> Result<HexBinary, ContractError> {
        let mut operators = make_operators_with_sigs(signers);
        operators.sort(); // gateway requires operators to be sorted

        let (addresses, weights, signatures): (Vec<_>, Vec<_>, Vec<_>) = operators
            .iter()
            .map(|op| {
                (
                    op.address.to_vec(),
                    u256_to_u128(op.weight),
                    op.signature.as_ref().map(|sig| sig.as_ref().to_vec()),
                )
            })
            .multiunzip();

        let signatures: Vec<Vec<u8>> = signatures.into_iter().flatten().collect();
        let quorum = u256_to_u128(quorum);
        Ok(bcs::to_bytes(&(addresses, weights, quorum, signatures))?.into())
    }

    fn make_command_id(command_id: &HexBinary) -> [u8; 32] {
        // command-ids are fixed length sequences
        command_id
            .to_vec()
            .try_into()
            .expect("couldn't convert command id to 32 byte array")
    }

    fn make_operators_with_sigs(
        signers_with_sigs: Vec<(Signer, Option<Signature>)>,
    ) -> Vec<Operator> {
        signers_with_sigs
            .into_iter()
            .map(|(signer, sig)| Operator {
                address: signer.pub_key.into(),
                weight: signer.weight,
                signature: sig,
            })
            .collect()
    }

    fn u256_to_u64(chain_id: Uint256) -> u64 {
        chain_id
            .to_string()
            .parse()
            .expect("value is larger than u64")
    }

    fn u256_to_u128(val: Uint256) -> u128 {
        val.to_string().parse().expect("value is larger than u128")
    }
}

#[cfg(test)]
mod tests {

    use libsecp256k1::{verify, RecoveryId};
    use solana_program::secp256k1_recover::secp256k1_recover;

    use super::*;
    use crate::primitives;

    #[test]
    fn can_create_execute_data() {
        let encode_data = create_execute_data(1, 2, 1);
        assert!(encode_data.is_ok())
    }

    fn test_data() -> (
        SecretKey,
        PublicKey,
        Message,
        libsecp256k1::Signature,
        RecoveryId,
    ) {
        // Create keypair
        let secret_key = SecretKey::random(&mut rand_core::OsRng);
        let public_key = PublicKey::from_secret_key(&secret_key);

        // Sign
        let message_array = primitives::array32();
        let message = Message::parse(&message_array);
        let (signature, recovery_id) = sign(&message, &secret_key);

        // Confidence check: self verify
        assert!(verify(&message, &signature, &public_key));

        (secret_key, public_key, message, signature, recovery_id)
    }

    #[test]
    fn solana_recovery_uncompressed() -> anyhow::Result<()> {
        let (_secret_key, public_key, message, signature, recovery_id) = test_data();

        // Recover
        let recovered_public_key = secp256k1_recover(
            &message.serialize(),
            recovery_id.serialize(),
            &signature.serialize(),
        )?;

        // Parse to compare
        let parsed_recovered_public_key =
            PublicKey::parse_slice(&recovered_public_key.to_bytes(), None)?;
        assert_eq!(parsed_recovered_public_key, public_key);
        Ok(())
    }

    #[test]
    fn solana_recovery_compressed() -> anyhow::Result<()> {
        let (_secret_key, public_key, message, signature, recovery_id) = test_data();

        // Recover
        let recovered_public_key_bytes = secp256k1_recover(
            &message.serialize(),
            recovery_id.serialize(),
            &signature.serialize(),
        )?
        .to_bytes();

        let recovered = PublicKey::parse_slice(&recovered_public_key_bytes, None)?;

        assert_eq!(
            recovered.serialize_compressed(),
            public_key.serialize_compressed()
        );

        Ok(())
    }
}
