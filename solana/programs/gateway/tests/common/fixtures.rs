use std::iter::repeat_with;

use anyhow::{anyhow, Result};
use axelar_wasm_std::nonempty::Uint256;
use axelar_wasm_std::Participant;
use connection_router::state::Address;
use connection_router::Message;
use cosmwasm_std::Addr;
use k256::ecdsa::SigningKey;
use multisig::key::{PublicKey, Signature};
use multisig::worker_set::WorkerSet;
use multisig_prover::encoding::{CommandBatchBuilder, Encoder};
use multisig_prover::types::CommandBatch;

struct Signer {
    address: Addr,
    weight: Uint256,
    signing_key: SigningKey,
    public_key: PublicKey,
}

impl Into<Participant> for Signer {
    fn into(self) -> Participant {
        Participant {
            address: self.address,
            weight: self.weight,
        }
    }
}

pub fn create_execute_data(
    num_messages: usize,
    num_signers: usize,
    threshold: u64,
) -> Result<Vec<u8>> {
    let messages: Vec<Message> = (0..num_messages)
        .map(|_| random_message())
        .collect::<Result<_, _>>()?;
    let signers: Vec<Signer> = (0..num_signers)
        .map(|_| create_signer())
        .collect::<Result<_, _>>()?;
    let command_batch: CommandBatch = create_command_batch(&messages, &signers, threshold)?;
    let signatures: Vec<Option<Signature>> = sign_batch(&command_batch, &signers);
    encode(&command_batch, &signers, &signatures)
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

fn create_command_batch(
    messages: &[Message],
    signers: &[Signer],
    threshold: u64,
) -> Result<CommandBatch> {
    let participants: Vec<(Participant, PublicKey)> = signers
        .iter()
        .map(|signer| -> Result<(Participant, PublicKey)> {
            let participant = Participant {
                address: signer.address.clone(),
                weight: signer.weight.try_into()?,
            };
            Ok((participant, signer.public_key.clone()))
        })
        .collect::<Result<_, _>>()?;

    let worker_set = WorkerSet::new(participants, threshold.into(), 0);

    let mut builder = CommandBatchBuilder::new(555u64.into(), Encoder::Bcs);
    for msg in messages {
        builder.add_message(msg.clone())?;
    }
    builder.add_new_worker_set(worker_set)?;

    Ok(builder.build()?)
}

fn create_signer() -> Result<Signer> {
    let signing_key = SigningKey::from_slice(&bytes(100))?;
    let verifying_key = *signing_key.verifying_key();

    Ok(Signer {
        signing_key,
        public_key: todo!(),
        address: addr(),
        weight: cosmwasm_std::Uint256::one().try_into()?,
    })
}

fn sign_batch(command_batch: &CommandBatch, signers: &[Signer]) -> Vec<Option<Signature>> {
    todo!()
}

fn encode(
    command_batch: &CommandBatch,
    signers: &[Signer],
    signatures: &[Option<Signature>],
) -> Result<Vec<u8>> {
    todo!()
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
    string(20).parse().map_err(|_| anyhow!("bad test naddress"))
}

fn addr() -> Addr {
    Addr::unchecked(string(20))
}

#[test]
fn msg() {
    let m = random_message();
    dbg!(m);
    panic!()
}
