use anchor_lang::prelude::*;

pub(crate) mod internal;

#[account]
pub struct State {
    pub value: bool,
    pub bump: u8,
}
