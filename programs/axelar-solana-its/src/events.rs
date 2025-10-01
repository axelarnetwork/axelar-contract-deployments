#![allow(missing_docs)]
use anchor_discriminators::Discriminator;
use event_cpi_macros::event;
use solana_program::pubkey::Pubkey;

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterchainTransfer {
    pub token_id: [u8; 32],
    pub source_address: Pubkey,
    pub source_token_account: Pubkey,
    pub destination_chain: String,
    pub destination_address: Vec<u8>,
    pub amount: u64,
    pub data_hash: [u8; 32],
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterchainTransferReceived {
    pub command_id: [u8; 32],
    pub token_id: [u8; 32],
    pub source_chain: String,
    pub source_address: Vec<u8>,
    pub destination_address: Pubkey,
    pub destination_token_account: Pubkey,
    pub amount: u64,
    pub data_hash: [u8; 32],
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenMetadataRegistered {
    pub token_address: Pubkey,
    pub decimals: u8,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkTokenStarted {
    pub token_id: [u8; 32],
    pub destination_chain: String,
    pub source_token_address: Pubkey,
    pub destination_token_address: Vec<u8>,
    pub token_manager_type: u8,
    pub params: Vec<u8>,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterchainTokenDeploymentStarted {
    pub token_id: [u8; 32],
    pub token_name: String,
    pub token_symbol: String,
    pub token_decimals: u8,
    pub minter: Vec<u8>,
    pub destination_chain: String,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenManagerDeployed {
    pub token_id: [u8; 32],
    pub token_manager: Pubkey,
    pub token_manager_type: u8,
    pub params: Vec<u8>,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterchainTokenDeployed {
    pub token_id: [u8; 32],
    pub token_address: Pubkey,
    pub minter: Pubkey,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterchainTokenIdClaimed {
    pub token_id: [u8; 32],
    pub deployer: Pubkey,
    pub salt: [u8; 32],
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeployRemoteInterchainTokenApproval {
    pub minter: Pubkey,
    pub deployer: Pubkey,
    pub token_id: [u8; 32],
    pub destination_chain: String,
    pub destination_minter: Vec<u8>,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RevokeRemoteInterchainTokenApproval {
    pub minter: Pubkey,
    pub deployer: Pubkey,
    pub token_id: [u8; 32],
    pub destination_chain: String,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FlowLimitSet {
    pub token_id: [u8; 32],
    pub operator: Pubkey,
    pub flow_limit: u64,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TrustedChainSet {
    pub chain_name: String,
}

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TrustedChainRemoved {
    pub chain_name: String,
}
