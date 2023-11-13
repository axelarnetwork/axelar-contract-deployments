use anyhow::Result;
use log::error;
use std::ops::Deref;

use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::{read_keypair_file, Signer};
use anchor_client::{Client, ClientError, Cluster};
use gateway::instructions::ContractCallEvent;
use solana_sdk::signature::Keypair;

pub mod client;
pub mod gateway_call_contract;
