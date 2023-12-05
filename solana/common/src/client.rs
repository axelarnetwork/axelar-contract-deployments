use super::log_parsing::parse_logs_from_contract_call_event;
use anyhow::{Error, Result};
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    signature::Signature,
};
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;

pub struct Client<'a> {
    pub rpc: &'a RpcClient,
    commitment: &'a str,
}

impl<'a> Client<'a> {
    pub fn new_without_payer(rpc: &'a RpcClient, commitment: &'a str) -> Self {
        Client { rpc, commitment }
    }

    // fetch_events_by_tx_signature returns vector of events for Gateway::ContractCall
    pub fn fetch_events_by_tx_signature_contract_call(
        &self,
        tx_id: Signature,
    ) -> Result<Vec<Vec<Vec<u8>>>, Error> {
        let tx_body = match self
            .rpc
            .get_transaction(&tx_id, UiTransactionEncoding::Base64)
        {
            Ok(v) => v,
            Err(e) => return Err(e.into()), // hack?
        };

        let tx_parsed_events = parse_logs_from_contract_call_event(tx_body, &gateway::id());

        Ok(tx_parsed_events)
    }

    // fetch_tx_signatures_per_address returns transactions for given contract address
    // tx_limit determine max size of the batch with txids
    pub fn fetch_tx_signatures_per_address(
        &self,
        contract_id: &Pubkey,
        before_tx: Option<Signature>,
        until_tx: Option<Signature>,
        tx_limit: usize,
    ) -> Vec<RpcConfirmedTransactionStatusWithSignature> {
        let commitment_level = CommitmentLevel::from_str(self.commitment).unwrap();
        let config_for_tx_fetch = GetConfirmedSignaturesForAddress2Config {
            before: before_tx,
            until: until_tx,
            limit: Some(tx_limit),
            commitment: Some(CommitmentConfig {
                commitment: commitment_level,
            }),
        };

        self.rpc
            .get_signatures_for_address_with_config(contract_id, config_for_tx_fetch)
            .unwrap()
    }
}
