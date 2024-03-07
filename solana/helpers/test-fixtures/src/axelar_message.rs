use std::ops::Deref;

use anyhow::{anyhow, Result};
use axelar_message_primitives::{DataPayload, DestinationProgramId};
use connection_router::state::Address;
use connection_router::Message;

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
