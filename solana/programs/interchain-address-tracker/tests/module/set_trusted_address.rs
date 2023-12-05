use interchain_address_tracker::get_associated_trusted_address;
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

use crate::utils::TestFixture;

#[tokio::test]
async fn test_set_trusted_address() {
    let mut test_fixture = TestFixture::new().await;

    let address_combos = [
        ("Ethereum-1", "0x1234567890123456789012345678901234567890"),
        ("Cosmos-1", "cosmos1abcdefg"),
        ("Solana-1", "111"),
    ];
    let rent = test_fixture.banks_client.get_rent().await.unwrap();
    let expected_len = interchain_address_tracker::state::RegisteredTrustedAddressAccount::LEN;
    let expected_balance = rent.minimum_balance(expected_len);

    for (trusted_chain_name, trusted_chain_address) in address_combos.iter() {
        let associated_trusted_address = get_associated_trusted_address(
            &test_fixture.associated_chain_address,
            trusted_chain_name,
        );
        let recent_blockhash = test_fixture
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap();

        // Associated account does not exist
        assert_eq!(
            test_fixture
                .banks_client
                .get_account(associated_trusted_address)
                .await
                .expect("get_account"),
            None,
        );

        let ix = interchain_address_tracker::instruction::build_set_trusted_address_instruction(
            &test_fixture.payer.pubkey(),
            &test_fixture.associated_chain_address,
            &associated_trusted_address,
            &test_fixture.owner.pubkey(),
            trusted_chain_name.to_string(),
            trusted_chain_address.to_string(),
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&test_fixture.payer.pubkey()),
            &[&test_fixture.payer, &test_fixture.owner],
            recent_blockhash,
        );

        test_fixture
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    // Associated accounts now exists
    for (trusted_chain_name, trusted_chain_address) in address_combos.iter() {
        let associated_trusted_address = get_associated_trusted_address(
            &test_fixture.associated_chain_address,
            trusted_chain_name,
        );

        // Associated account now exists
        let associated_account = test_fixture
            .banks_client
            .get_account(associated_trusted_address)
            .await
            .expect("get_account")
            .expect("associated_account not none");
        assert_eq!(associated_account.owner, interchain_address_tracker::id());
        assert_eq!(
            associated_account.data.len(),
            interchain_address_tracker::state::RegisteredTrustedAddressAccount::LEN
        );
        assert_eq!(associated_account.lamports, expected_balance);
        let account_info =
            interchain_address_tracker::state::RegisteredTrustedAddressAccount::unpack_from_slice(
                associated_account.data.as_slice(),
            )
            .unwrap();
        assert_eq!(account_info.address, trusted_chain_address.to_string());
    }
}
