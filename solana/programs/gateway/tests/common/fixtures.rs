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

pub fn create_execute_data(num_messages: usize, num_signers: usize) -> Result<Vec<u8>> {
    let messages: Vec<Message> = (0..num_messages)
        .map(|_| random_message())
        .collect::<Result<_, _>>()?;
    let signers: Vec<TestSigner> = (0..num_signers)
        .map(|_| create_signer())
        .collect::<Result<_, _>>()?;
    let command_batch = create_command_batch(&messages)?;
    let signatures: Vec<Option<Signature>> = sign_batch(&command_batch, &signers)?;
    encode(&command_batch, signers, signatures)
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
) -> Result<Vec<u8>> {
    ensure!(
        signers.len() == signatures.len(),
        "signers and signature missmatch"
    );
    let signers_and_signatures: Vec<(Signer, Option<Signature>)> = signers
        .into_iter()
        .map(Into::into)
        .zip(signatures.into_iter())
        .collect();

    todo!("finish encoding")
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

#[test]
fn msg() -> Result<()> {
    create_execute_data(5, 3).unwrap();
    bail!("finish this")
}
