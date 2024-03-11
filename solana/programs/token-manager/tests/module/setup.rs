use gateway::state::GatewayConfig;
use solana_program::program_option::COption;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use spl_associated_token_account::get_associated_token_address;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::test_setup::interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use test_fixtures::test_setup::interchain_token_transfer_gmp::Bytes32;
use test_fixtures::test_setup::TestFixture;

use crate::program_test;

#[tokio::test]
async fn test_token_manager_setup() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let mint_authority = Keypair::new();
    let interchain_token_service_root_pda = Keypair::new();
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Pubkey::from([0; 32]);
    let init_flow_manager = Pubkey::from([0; 32]);
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let gateway_root_config_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;
    let groups = fixture
        .derive_token_manager_permission_groups(
            &token_id,
            &interchain_token_service_root_pda.pubkey(),
            &init_flow_manager,
            &init_operator,
        )
        .await;
    fixture
        .setup_permission_group(&groups.flow_limiter_group)
        .await;
    fixture.setup_permission_group(&groups.operator_group).await;

    // Action
    let token_manager_pda_pubkey = fixture
        .setup_token_manager(
            token_manager::TokenManagerType::LockUnlock,
            groups,
            500,
            gateway_root_config_pda,
            token_mint,
            interchain_token_service_root_pda.pubkey(),
        )
        .await;

    // Assert
    let token_manager_pda = fixture
        .banks_client
        .get_account(token_manager_pda_pubkey)
        .await
        .expect("get_account")
        .expect("account not none");
    let data = token_manager_pda
        .check_initialized_pda::<token_manager::state::TokenManagerRootAccount>(&token_manager::ID)
        .unwrap();
    assert_eq!(
        data,
        token_manager::state::TokenManagerRootAccount {
            flow_limit: 500,
            associated_token_account: get_associated_token_address(
                &token_manager_pda_pubkey,
                &token_mint
            ),
            token_manager_type: token_manager::TokenManagerType::LockUnlock,
            token_mint
        }
    );
    let token_manager_ata = fixture
        .banks_client
        .get_account(get_associated_token_address(
            &token_manager_pda_pubkey,
            &token_mint,
        ))
        .await
        .expect("get_account")
        .expect("account not none");

    let data = spl_token::state::Account::unpack(token_manager_ata.data.as_slice()).unwrap();
    assert_eq!(
        data,
        spl_token::state::Account {
            mint: token_mint,
            owner: token_manager_pda_pubkey,
            amount: 0,
            delegate: COption::Some(interchain_token_service_root_pda.pubkey()),
            state: spl_token::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: u64::MAX,
            close_authority: COption::None,
        }
    );
}
