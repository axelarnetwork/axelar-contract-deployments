mod instructions;

use anchor_lang::prelude::*;
use instructions::*;

declare_id!("9fH45E6rSfwx71HEgDnmhR5fZwj3vvJpExRvHQbU6Z2X");

#[program]
pub mod registry {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, seeds_hash: [u8; 32], v: bool) -> Result<()> {
        instructions::initialize(ctx, seeds_hash, v).unwrap();
        Ok(())
    }

    pub fn set(ctx: Context<Set>, seeds_hash: [u8; 32], v: bool) -> Result<()> {
        instructions::set(ctx, seeds_hash, v).unwrap();
        Ok(())
    }

    pub fn get(ctx: Context<Get>, seeds_hash: [u8; 32]) -> Result<bool> {
        Ok(instructions::get(ctx, seeds_hash).unwrap())
    }

    pub fn delete(ctx: Context<Delete>, seeds_hash: [u8; 32]) -> Result<()> {
        instructions::delete(ctx, seeds_hash).unwrap();
        Ok(())
    }
}

#[account]
pub struct State {
    pub value: bool,
    pub authority: Pubkey,
}
