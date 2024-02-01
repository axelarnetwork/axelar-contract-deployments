use gateway::accounts::GatewayConfig;
use interchain_token_service::get_interchain_token_service_root_pda;
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

#[tokio::test]
async fn test_init_root_pda_interchain_token_service() {
    let mut fixture = super::utils::TestFixture::new().await;
    let gas_service_root_pda = fixture.init_gas_service().await;

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;

    let interchain_token_service_root_pda =
        get_interchain_token_service_root_pda(&gateway_root_pda, &gas_service_root_pda);

    let ix = interchain_token_service::instruction::build_initialize_instruction(
        &fixture.payer.pubkey(),
        &interchain_token_service_root_pda,
        &gateway_root_pda,
        &gas_service_root_pda,
    )
    .unwrap();
    let blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let interchain_token_service_root_pda = fixture
        .banks_client
        .get_account(interchain_token_service_root_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(
        interchain_token_service_root_pda.owner,
        interchain_token_service::id()
    );
    assert_eq!(
        interchain_token_service_root_pda.data.len(),
        interchain_token_service::state::RootPDA::LEN
    );
    let token_manager_account = interchain_token_service::state::RootPDA::unpack_from_slice(
        &interchain_token_service_root_pda.data,
    )
    .unwrap();
    assert_eq!(
        token_manager_account,
        interchain_token_service::state::RootPDA {}
    );
}
