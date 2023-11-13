use super::*;
use crate::common::contract_call::Params;

#[event]
pub struct ContractCallApprovedEvent {
    pub command_id: [u8; 32],
    pub source_chain: String,
    pub source_address: String,
    pub contract_address: String,
    pub payload_hash: [u8; 32],
    pub source_tx_hash: [u8; 32],
    pub source_event_index: [u8; 256],
}

fn approve(acc: &mut Account<'_, State>, params: Vec<u8>, command_id: [u8; 32]) -> Result<()> {
    let p = Params::decode(params);
    acc.value = true;

    emit!(ContractCallApprovedEvent {
        command_id,
        source_chain: p.source_chain,
        source_address: p.source_address,
        contract_address: p.contract_address,
        payload_hash: p.payload_hash,
        source_tx_hash: p.source_tx_hash,
        source_event_index: p.source_event_index
    });

    Ok(())
}
