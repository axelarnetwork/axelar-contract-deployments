use anyhow::{anyhow, Result};
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

fn address() -> Result<Address> {
    hex::encode(array32())
        .parse()
        .map_err(|_| anyhow!("bad test naddress"))
}
