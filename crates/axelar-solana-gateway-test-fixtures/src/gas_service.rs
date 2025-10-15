//! Utilities for working with the Axelar gas service

use crate::base::TestFixture;
use axelar_solana_gateway::BytemuckedPda;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::{
    account::ReadableAccount, program_pack::Pack, pubkey::Pubkey, signature::Keypair,
    signer::Signer, system_instruction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::{extension::ExtensionType, state::Mint};

/// Utility structure for keeping gas service related state
pub struct GasServiceUtils {
    /// upgrade authority of the program
    pub upgrade_authority: Keypair,
    /// gas service operator
    pub operator: Keypair,
    /// PDA of the gas service config
    pub config_pda: Pubkey,
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
        self.setup_default_gas_config(upgrade_authority)
    }

    /// Initialise a new gas config and return a utility tracker struct for it
    pub fn setup_default_gas_config(&mut self, upgrade_authority: Keypair) -> GasServiceUtils {
        let operator = Keypair::new();
        let (config_pda, ..) = axelar_solana_gas_service::get_config_pda();

        GasServiceUtils {
            upgrade_authority,
            operator,
            config_pda,
        }
    }

    /// init the gas service
    pub async fn init_gas_config(
        &mut self,
        utils: &GasServiceUtils,
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        self.init_gas_config_with_params(utils.operator.insecure_clone())
            .await
    }

    /// init the gas service with raw params
    pub async fn init_gas_config_with_params(
        &mut self,
        operator: Keypair,
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        let ix = axelar_solana_gas_service::instructions::init_config(
            &self.payer.pubkey(),
            &operator.pubkey(),
        )
        .unwrap();
        self.send_tx_with_custom_signers(&[ix], &[operator, self.payer.insecure_clone()])
            .await
    }

    /// Initialize a new token mint
    #[allow(clippy::disallowed_methods)]
    pub async fn init_new_mint(
        &mut self,
        mint_authority: Pubkey,
        token_program_id: Pubkey,
        decimals: u8,
    ) -> Pubkey {
        let mint_account = Keypair::new();
        let rent = self.get_rent(Mint::LEN).await;

        let instructions = &[
            system_instruction::create_account(
                &self.payer.pubkey(),
                &mint_account.pubkey(),
                rent,
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
        ];
        self.send_tx_with_custom_signers(
            instructions,
            &[&self.payer.insecure_clone(), &mint_account],
        )
        .await
        .unwrap();

        mint_account.pubkey()
    }

    /// Initialize a new token mint with a fee (uses `spl_token_2022`)
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::disallowed_methods)]
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
        let mint_account = Keypair::new();
        let space =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
                .unwrap();
        let rent = self.get_rent(space).await;

        let instructions = [
            system_instruction::create_account(
                &self.payer.pubkey(),
                &mint_account.pubkey(),
                rent,
                space.try_into().unwrap(),
                token_program_id,
            ),
            spl_token_2022::extension::transfer_fee::instruction::initialize_transfer_fee_config(
                token_program_id,
                &mint_account.pubkey(),
                transfer_fee_config_authority,
                withdraw_withheld_authority,
                fee_basis_points,
                maximum_fee,
            )
            .unwrap(),
            spl_token_2022::instruction::initialize_mint(
                token_program_id,
                &mint_account.pubkey(),
                mint_authority,
                None,
                decimals,
            )
            .unwrap(),
        ];
        self.send_tx_with_custom_signers(
            &instructions,
            &[&self.payer.insecure_clone(), &mint_account],
        )
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
        let ix = spl_token_2022::instruction::mint_to(
            token_program_id,
            mint,
            to,
            &mint_authority.pubkey(),
            &[&mint_authority.pubkey()],
            amount,
        )
        .unwrap();

        self.send_tx_with_custom_signers(&[ix], &[&self.payer.insecure_clone(), mint_authority])
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
        self.send_tx(&[ix]).await.unwrap();
        associated_account_address
    }

    /// get the data from a token account
    pub async fn get_token_account(
        &mut self,
        token_account: &Pubkey,
    ) -> spl_token_2022::state::Account {
        let res = self
            .try_get_account_no_checks(token_account)
            .await
            .unwrap()
            .unwrap();

        spl_token_2022::state::Account::unpack_from_slice(&res.data).unwrap()
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
