use anchor_lang::prelude::*;
use borsh::BorshSerialize;

#[event]
pub struct OperatorshipTransferredEvent {
    pub new_operators_data: Vec<u8>,
}
