use std::iter::repeat_with;

use anyhow::{anyhow, bail, ensure, Result};
use connection_router::state::Address;
use connection_router::Message;
use cosmwasm_std::{Addr, Uint256};
use k256::ecdsa::SigningKey;
use multisig::key::{KeyType, PublicKey, Signature};
use multisig::msg::Signer;
use multisig_prover::encoding::{CommandBatchBuilder, Encoder};
use multisig_prover::types::CommandBatch;

#[derive(Debug)]
struct TestSigner {
    address: Addr,
    weight: Uint256,
    signing_key: SigningKey,
    pub_key: PublicKey,
}

impl From<TestSigner> for Signer {
    fn from(val: TestSigner) -> Self {
        let TestSigner {
            address,
            weight,
            pub_key,
            ..
        } = val;
        Signer {
            address,
            weight,
            pub_key,
        }
    }
}

pub fn create_execute_data(
    num_messages: usize,
    num_signers: usize,
    quorum: u128,
) -> Result<Vec<u8>> {
    let messages: Vec<Message> = (0..num_messages)
        .map(|_| random_message())
        .collect::<Result<_, _>>()?;
    let signers: Vec<TestSigner> = (0..num_signers)
        .map(|_| create_signer())
        .collect::<Result<_, _>>()?;
    let command_batch = create_command_batch(&messages)?;
    let signatures: Vec<Option<Signature>> = sign_batch(&command_batch, &signers)?;
    encode(&command_batch, signers, signatures, quorum)
}

fn random_message() -> Result<Message> {
    let message = Message {
        cc_id: format!("{}:{}", string(10), string(10)).parse()?,
        source_address: address()?,
        destination_chain: string(10).parse()?,
        destination_address: address()?,
        payload_hash: array32(),
    };
    Ok(message)
}

fn create_command_batch(messages: &[Message]) -> Result<CommandBatch> {
    let mut builder = CommandBatchBuilder::new(555u64.into(), Encoder::Bcs);
    for msg in messages {
        builder.add_message(msg.clone())?;
    }
    Ok(builder.build()?)
}

fn create_signer() -> Result<TestSigner> {
    let signing_key = SigningKey::random(&mut rand_core::OsRng);

    let public_key: PublicKey = (
        KeyType::Ecdsa,
        signing_key.verifying_key().to_sec1_bytes().as_ref().into(),
    )
        .try_into()?;

    Ok(TestSigner {
        signing_key,
        pub_key: public_key,
        address: addr(),
        weight: cosmwasm_std::Uint256::one().try_into()?,
    })
}

fn sign_batch(
    command_batch: &CommandBatch,
    signers: &[TestSigner],
) -> Result<Vec<Option<Signature>>> {
    use k256::ecdsa::signature::Signer as _;
    use k256::ecdsa::{self};

    let message_to_sign = command_batch.msg_digest();
    signers
        .iter()
        .map(|signer| signer.signing_key.sign(&message_to_sign))
        .try_fold(vec![], |mut collected, signature: ecdsa::Signature| {
            match (KeyType::Ecdsa, signature.to_vec().into()).try_into() {
                Err(e) => bail!("failed to convert signature: {e}"),
                Ok(sig) => collected.push(Some(sig)),
            };
            Ok(collected)
        })
}

fn encode(
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
        .zip(signatures.into_iter())
        .collect();

    axelar_bcs_encoding::encode_execute_data(command_batch, quorum, signers_and_signatures)
        .map_err(|e| anyhow!("failed to encode execute_data: {e}"))
        .map(|hexbinary| hexbinary.to_vec())
}

fn string(n: usize) -> String {
    repeat_with(fastrand::alphanumeric).take(n).collect()
}

fn bytes(n: usize) -> Vec<u8> {
    repeat_with(|| fastrand::u8(..)).take(n).collect()
}

fn array32() -> [u8; 32] {
    bytes(32).try_into().unwrap()
}

fn address() -> Result<Address> {
    hex::encode(array32())
        .parse()
        .map_err(|_| anyhow!("bad test naddress"))
}

fn addr() -> Addr {
    Addr::unchecked(string(20))
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
