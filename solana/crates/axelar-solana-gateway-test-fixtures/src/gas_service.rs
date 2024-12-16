//! Utilities for working with the Axelar gas service

use crate::base::TestFixture;
use axelar_solana_gas_service::processor::GasServiceEvent;
use axelar_solana_gateway::BytemuckedPda;
use gateway_event_stack::{MatchContext, ProgramInvocationState};
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::{
    account::ReadableAccount, keccak, pubkey::Pubkey, signature::Keypair, signer::Signer,
};

/// Utility structure for keeping gas service related state
pub struct GasServiceUtils {
    /// upgrade authority of the program
    pub upgrade_authority: Keypair,
    /// the config authorty
    pub config_authority: Keypair,
    /// PDA of the gas service config
    pub config_pda: Pubkey,
    /// salt to derive the config pda
    pub salt: [u8; 32],
}

impl TestFixture {
    /// Deploy the gas service program and construct a pre-emptive
    pub async fn deploy_gas_service(&mut self) -> GasServiceUtils {
        // deploy gas service
        let gas_service_bytecode =
            tokio::fs::read("../../target/deploy/axelar_solana_gas_service.so")
                .await
                .unwrap();

        // Generate a new keypair for the upgrade authority
        let upgrade_authority = Keypair::new();

        self.register_upgradeable_program(
            &gas_service_bytecode,
            &upgrade_authority.pubkey(),
            &axelar_solana_gas_service::id(),
        )
        .await;

        let config_authority = Keypair::new();
        let salt = keccak::hash(b"my gas service").0;
        let (config_pda, ..) = axelar_solana_gas_service::get_config_pda(
            &axelar_solana_gas_service::ID,
            &salt,
            &config_authority.pubkey(),
        );

        GasServiceUtils {
            upgrade_authority,
            config_authority,
            config_pda,
            salt,
        }
    }

    /// init the gas service
    pub async fn init_gas_config(
        &mut self,
        utils: &GasServiceUtils,
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        self.init_gas_config_with_params(
            utils.config_authority.pubkey(),
            utils.config_pda,
            utils.salt,
        )
        .await
    }

    /// init the gas service with raw params
    pub async fn init_gas_config_with_params(
        &mut self,
        config_authority: Pubkey,
        config_pda: Pubkey,
        salt: [u8; 32],
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        let ix = axelar_solana_gas_service::instructions::init_config(
            &axelar_solana_gas_service::ID,
            &self.payer.pubkey(),
            &config_authority,
            &config_pda,
            salt,
        )
        .unwrap();
        self.send_tx(&[ix]).await
    }

    /// get the gas service config pda state
    pub async fn gas_service_config_state(
        &mut self,
        config_pda: Pubkey,
    ) -> axelar_solana_gas_service::state::Config {
        let acc = self
            .get_account(&config_pda, &axelar_solana_gas_service::ID)
            .await;
        let config = axelar_solana_gas_service::state::Config::read(acc.data()).unwrap();
        *config
    }
}

/// Get events emitted by the `GasService`
#[must_use]
pub fn get_gas_service_events(
    tx: &solana_program_test::BanksTransactionResultWithMetadata,
) -> Vec<ProgramInvocationState<GasServiceEvent>> {
    let match_context = MatchContext::new(&axelar_solana_gas_service::ID.to_string());
    gateway_event_stack::build_program_event_stack(
        &match_context,
        tx.metadata.as_ref().unwrap().log_messages.as_slice(),
        gateway_event_stack::parse_gas_service_log,
    )
}
