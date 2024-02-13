use account_group::instruction::GroupId;
use account_group::{get_permission_account, get_permission_group_account};
use borsh::BorshDeserialize;
use gateway::accounts::GatewayConfig;
use interchain_token_service::{
    get_flow_limiters_permission_group_id, get_interchain_token_service_root_pda,
    get_operators_permission_group_id,
};
pub use interchain_token_transfer_gmp;
use interchain_token_transfer_gmp::ethers_core::types::U256;
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::{Bytes32, DeployTokenManager};
use solana_program::clock::Clock;
use solana_program::hash::Hash;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_program_test::{BanksClient, ProgramTest, ProgramTestBanksClientExt};
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_token::state::Mint;
use token_manager::state::TokenManagerRootAccount;
use token_manager::{get_token_manager_account, CalculatedEpoch, TokenManagerType};

use crate::account::CheckValidPDAInTests;

pub struct TestFixture {
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub recent_blockhash: Hash,
}

impl TestFixture {
    pub async fn new(pt: ProgramTest) -> TestFixture {
        let (banks_client, payer, recent_blockhash) = pt.start().await;
        TestFixture {
            banks_client,
            payer,
            recent_blockhash,
        }
    }

    pub async fn refresh_blockhash(&mut self) -> Hash {
        self.recent_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&self.recent_blockhash)
            .await
            .unwrap();
        self.recent_blockhash
    }

    pub async fn init_gas_service(&mut self) -> Pubkey {
        let (root_pda_address, _) = gas_service::get_gas_service_root_pda();
        let ix =
            gas_service::instruction::create_initialize_root_pda_ix(self.payer.pubkey()).unwrap();
        self.recent_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&self.recent_blockhash)
            .await
            .unwrap();

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.recent_blockhash,
        );

        self.banks_client.process_transaction(tx).await.unwrap();

        let root_pda_data = self
            .banks_client
            .get_account(root_pda_address)
            .await
            .unwrap()
            .unwrap();
        let root_pda_data =
            gas_service::accounts::GasServiceRootPDA::try_from_slice(root_pda_data.data.as_slice())
                .unwrap();

        assert!(root_pda_data.check_authority(self.payer.pubkey().into()));

        root_pda_address
    }

    pub async fn initialize_gateway_config_account(
        &mut self,
        gateway_config: GatewayConfig,
    ) -> Pubkey {
        self.recent_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&self.recent_blockhash)
            .await
            .unwrap();
        let (gateway_config_pda, _bump) = GatewayConfig::pda();

        let ix =
            gateway::instructions::initialize_config(self.payer.pubkey(), gateway_config.clone())
                .unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.recent_blockhash,
        );

        self.banks_client.process_transaction(tx).await.unwrap();

        let account = self
            .banks_client
            .get_account(gateway_config_pda)
            .await
            .unwrap()
            .expect("metadata");

        assert_eq!(account.owner, gateway::id());
        let deserialized_gateway_config: GatewayConfig = borsh::from_slice(&account.data).unwrap();
        assert_eq!(deserialized_gateway_config, gateway_config);

        gateway_config_pda
    }

    pub async fn init_its_root_pda(
        &mut self,
        gateway_root_pda: &Pubkey,
        gas_service_root_pda: &Pubkey,
    ) -> Pubkey {
        let interchain_token_service_root_pda =
            get_interchain_token_service_root_pda(gateway_root_pda, gas_service_root_pda);
        let ix = interchain_token_service::instruction::build_initialize_instruction(
            &self.payer.pubkey(),
            &interchain_token_service_root_pda,
            gateway_root_pda,
            gas_service_root_pda,
        )
        .unwrap();
        let blockhash = self.refresh_blockhash().await;
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
        interchain_token_service_root_pda
    }

    pub async fn derive_token_manager_permission_groups(
        &self,
        token_id: &Bytes32,
        interchain_token_service_root_pda: &Pubkey,
        // In most cases this will be the same as `interchain_token_service_root_pda`
        init_flow_limiter: &Pubkey,
        init_operator: &Pubkey,
    ) -> ITSTokenHandlerGroups {
        let operator_group_id =
            get_operators_permission_group_id(token_id, interchain_token_service_root_pda);
        let operator_group_pda = get_permission_group_account(&operator_group_id);
        let init_operator_pda_acc = get_permission_account(&operator_group_pda, init_operator);

        let flow_group_id =
            get_flow_limiters_permission_group_id(token_id, interchain_token_service_root_pda);
        let flow_group_pda = get_permission_group_account(&flow_group_id);
        let init_flow_pda_acc = get_permission_account(&flow_group_pda, init_flow_limiter);

        ITSTokenHandlerGroups {
            operator_group: PermissionGroup {
                id: operator_group_id,
                group_pda: operator_group_pda,
                group_pda_user: init_operator_pda_acc,
                group_pda_user_owner: *init_operator,
            },
            flow_limiter_group: PermissionGroup {
                id: flow_group_id,
                group_pda: flow_group_pda,
                group_pda_user: init_flow_pda_acc,
                group_pda_user_owner: *init_flow_limiter,
            },
        }
    }

    pub async fn init_new_mint(&mut self, mint_authority: Pubkey) -> Pubkey {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let mint_account = Keypair::new();
        let rent = self.banks_client.get_rent().await.unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &self.payer.pubkey(),
                    &mint_account.pubkey(),
                    rent.minimum_balance(Mint::LEN),
                    Mint::LEN as u64,
                    &spl_token::id(),
                ),
                spl_token::instruction::initialize_mint(
                    &spl_token::id(),
                    &mint_account.pubkey(),
                    &mint_authority,
                    None,
                    0,
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

    pub async fn mint_tokens_to(
        &mut self,
        mint: Pubkey,
        to: Pubkey,
        mint_authority: Keypair,
        amount: u64,
    ) {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint,
            &to,
            &mint_authority.pubkey(),
            &[&mint_authority.pubkey()],
            amount,
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, &mint_authority],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    pub async fn init_new_token_manager(
        &mut self,
        interchain_token_service_root_pda: Pubkey,
        token_mint: Pubkey,
        gateway_root_pda: Pubkey,
        token_manager_type: TokenManagerType,
    ) -> (Pubkey, TokenManagerRootAccount, ITSTokenHandlerGroups) {
        let token_id = Bytes32(keccak256("random-token-id"));
        let init_operator = Pubkey::from([0; 32]);
        let init_flow_limiter = Pubkey::from([0; 32]);

        let its_token_manager_permission_groups = self
            .derive_token_manager_permission_groups(
                &token_id,
                &interchain_token_service_root_pda,
                &init_flow_limiter,
                &init_operator,
            )
            .await;
        let token_manager_root_pda_pubkey = get_token_manager_account(
            &its_token_manager_permission_groups.operator_group.group_pda,
            &its_token_manager_permission_groups
                .flow_limiter_group
                .group_pda,
            &interchain_token_service_root_pda,
        );

        // Action
        let ix = interchain_token_service::instruction::build_deploy_token_manager_instruction(
            &self.payer.pubkey(),
            &token_manager_root_pda_pubkey,
            &its_token_manager_permission_groups.operator_group.group_pda,
            &its_token_manager_permission_groups
                .operator_group
                .group_pda_user_owner,
            &its_token_manager_permission_groups
                .flow_limiter_group
                .group_pda,
            &its_token_manager_permission_groups
                .flow_limiter_group
                .group_pda_user_owner,
            &interchain_token_service_root_pda,
            &token_mint,
            &gateway_root_pda,
            DeployTokenManager {
                token_id: Bytes32(keccak256("random-token-id")),
                token_manager_type: U256::from(token_manager_type as u8),
                params: vec![],
            },
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.banks_client.get_latest_blockhash().await.unwrap(),
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
        let token_manager_data = self
            .banks_client
            .get_account(token_manager_root_pda_pubkey)
            .await
            .expect("get_account")
            .expect("account not none");
        let data = token_manager_data
            .check_initialized_pda::<token_manager::state::TokenManagerRootAccount>(
                &token_manager::ID,
            )
            .unwrap();
        (
            token_manager_root_pda_pubkey,
            data,
            its_token_manager_permission_groups,
        )
    }

    /// Returns token manager root pda
    pub async fn setup_token_manager(
        &mut self,
        token_manager_type: TokenManagerType,
        groups: ITSTokenHandlerGroups,
        flow_limit: u64,
        gateway_root_config_pda: Pubkey,
        token_mint: Pubkey,
        its_pda: Pubkey,
    ) -> Pubkey {
        let token_manager_pda = token_manager::get_token_manager_account(
            &groups.operator_group.group_pda,
            &groups.flow_limiter_group.group_pda,
            &its_pda,
        );
        let clock = self.banks_client.get_sysvar::<Clock>().await.unwrap();
        let block_timestamp = clock.unix_timestamp;

        let _token_flow_pda = token_manager::get_token_flow_account(
            &token_manager_pda,
            CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
        );

        self.recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = token_manager::instruction::build_setup_instruction(
            &self.payer.pubkey(),
            &token_manager_pda,
            &groups.operator_group.group_pda,
            &groups.operator_group.group_pda_user_owner,
            &groups.flow_limiter_group.group_pda,
            &groups.flow_limiter_group.group_pda_user_owner,
            &its_pda,
            &token_mint,
            &gateway_root_config_pda,
            token_manager::instruction::Setup {
                flow_limit,
                token_manager_type,
            },
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        token_manager_pda
    }

    pub async fn setup_permission_group(&mut self, group: &PermissionGroup) {
        let ix = account_group::instruction::build_setup_permission_group_instruction(
            &self.payer.pubkey(),
            &group.group_pda,
            &group.group_pda_user,
            &group.group_pda_user_owner,
            group.id.clone(),
        )
        .unwrap();
        self.recent_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&self.recent_blockhash)
            .await
            .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct PermissionGroup {
    pub id: GroupId,
    pub group_pda: Pubkey,
    pub group_pda_user: Pubkey,
    pub group_pda_user_owner: Pubkey,
}

#[derive(Debug, Clone)]
pub struct ITSTokenHandlerGroups {
    pub operator_group: PermissionGroup,
    pub flow_limiter_group: PermissionGroup,
}
