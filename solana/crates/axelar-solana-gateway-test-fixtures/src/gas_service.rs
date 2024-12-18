//! Utilities for working with the Axelar gas service

use crate::base::TestFixture;
use axelar_solana_gas_service::processor::GasServiceEvent;
use axelar_solana_gateway::BytemuckedPda;
use gateway_event_stack::{MatchContext, ProgramInvocationState};
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::{
    account::ReadableAccount, keccak, program_pack::Pack, pubkey::Pubkey, signature::Keypair,
    signer::Signer, system_instruction, transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::{extension::ExtensionType, state::Mint};

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

    /// Initialize a new token mint
    pub async fn init_new_mint(
        &mut self,
        mint_authority: Pubkey,
        token_program_id: Pubkey,
        decimals: u8,
    ) -> Pubkey {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let mint_account = Keypair::new();
        let rent = self.banks_client.get_rent().await.unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &self.payer.pubkey(),
                    &mint_account.pubkey(),
                    rent.minimum_balance(Mint::LEN),
                    Mint::LEN.try_into().unwrap(),
                    &token_program_id,
                ),
                spl_token_2022::instruction::initialize_mint(
                    &token_program_id,
                    &mint_account.pubkey(),
                    &mint_authority,
                    None,
                    decimals,
                )
                .unwrap(),
            ],
            Some(&self.payer.pubkey()),
            &[&self.payer, &mint_account],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        mint_account.pubkey()
    }

    /// Initialize a new token mint with a fee (uses `spl_token_2022`)
    #[allow(clippy::too_many_arguments)]
    pub async fn init_new_mint_with_fee(
        &mut self,
        mint_authority: &Pubkey,
        token_program_id: &Pubkey,
        fee_basis_points: u16,
        maximum_fee: u64,
        decimals: u8,
        transfer_fee_config_authority: Option<&Pubkey>,
        withdraw_withheld_authority: Option<&Pubkey>,
    ) -> Pubkey {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let mint_account = Keypair::new();
        let rent = self.banks_client.get_rent().await.unwrap();
        let space =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
                .unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &self.payer.pubkey(),
                    &mint_account.pubkey(),
                    rent.minimum_balance(space),
                    space.try_into().unwrap(),
                    token_program_id,
                ),
                spl_token_2022::extension::transfer_fee::instruction::initialize_transfer_fee_config(
                    token_program_id,
                    &mint_account.pubkey(),
                    transfer_fee_config_authority,
                    withdraw_withheld_authority,
                    fee_basis_points,
                    maximum_fee
                ).unwrap(),
                spl_token_2022::instruction::initialize_mint(
                    token_program_id,
                    &mint_account.pubkey(),
                    mint_authority,
                    None,
                    decimals
                )
                .unwrap(),
            ],
            Some(&self.payer.pubkey()),
            &[&self.payer, &mint_account],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        mint_account.pubkey()
    }

    /// mint tokents to someones token account
    pub async fn mint_tokens_to(
        &mut self,
        mint: &Pubkey,
        to: &Pubkey,
        mint_authority: &Keypair,
        amount: u64,
        token_program_id: &Pubkey,
    ) {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = spl_token_2022::instruction::mint_to(
            token_program_id,
            mint,
            to,
            &mint_authority.pubkey(),
            &[&mint_authority.pubkey()],
            amount,
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, mint_authority],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    /// init a new ATA account
    pub async fn init_associated_token_account(
        &mut self,
        token_mint_address: &Pubkey,
        holder_wallet_address: &Pubkey,
        token_program_id: &Pubkey,
    ) -> Pubkey {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let associated_account_address = get_associated_token_address_with_program_id(
            holder_wallet_address,
            token_mint_address,
            token_program_id,
        );
        let ix = spl_associated_token_account::instruction::create_associated_token_account(
            &self.payer.pubkey(),
            holder_wallet_address,
            token_mint_address,
            token_program_id,
        );

        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
        associated_account_address
    }

    /// get the data from a token account
    pub async fn get_token_account(
        &mut self,
        token_account: &Pubkey,
    ) -> spl_token_2022::state::Account {
        self.banks_client
            .get_packed_account_data::<spl_token_2022::state::Account>(*token_account)
            .await
            .unwrap()
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
