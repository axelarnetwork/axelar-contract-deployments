use std::ops::Deref;

use anyhow::{anyhow, Result};
use axelar_message_primitives::{DataPayload, DestinationProgramId};
use axelar_wasm_std::{nonempty, Participant};
use connection_router::state::Address;
use connection_router::Message;
use cosmwasm_std::{Addr, Uint256};
use multisig::worker_set::WorkerSet;

use crate::execute_data::TestSigner;
use crate::primitives::{array32, string};

pub fn message() -> Result<Message> {
    let message = Message {
        cc_id: format!("{}:{}", string(10), string(10)).parse()?,
        source_address: address()?,
        destination_chain: string(10).parse()?,
        destination_address: address()?,
        payload_hash: array32(),
    };
    Ok(message)
}

pub fn new_worker_set(
    participants: &[TestSigner],
    created_at_block: u64,
    new_threshold: Uint256,
) -> WorkerSet {
    let participants = participants
        .iter()
        .map(|p| {
            let public_key = p.public_key.clone();
            let participant = Participant {
                weight: nonempty::Uint256::try_from(p.weight).unwrap(),
                address: Addr::unchecked(hex::encode(&p.public_key)),
            };
            (participant, public_key)
        })
        .collect::<Vec<_>>();

    WorkerSet::new(participants, new_threshold, created_at_block)
}

pub fn custom_message(
    destination_pubkey: impl Into<DestinationProgramId>,
    payload: DataPayload<'_>,
) -> Result<Message> {
    let payload_hash = payload.hash();
    let destination_pubkey = destination_pubkey.into();

    let message = Message {
        cc_id: format!("{}:{}", string(10), string(10)).parse()?,
        source_address: address()?,
        destination_chain: string(10).parse()?,
        destination_address: hex::encode(destination_pubkey.0.to_bytes())
            .parse()
            .map_err(|_| anyhow!("bad test destination_address"))?,
        payload_hash: *payload_hash.0.deref(),
    };
    Ok(message)
}

fn address() -> Result<Address> {
    hex::encode(array32())
        .parse()
        .map_err(|_| anyhow!("bad test naddress"))
}
