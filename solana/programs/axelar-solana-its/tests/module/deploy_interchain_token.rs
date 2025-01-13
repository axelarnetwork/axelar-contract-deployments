use axelar_solana_its::instructions::DeployInterchainTokenInputs;
use solana_program_test::tokio;
use solana_sdk::keccak;
use solana_sdk::signer::Signer;

use crate::{axelar_solana_setup, ItsProgramWrapper};

#[rstest::rstest]
#[tokio::test]
async fn test_deploy_interchain_token() {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;
    let gas_utils = solana_chain.fixture.deploy_gas_service().await;
    solana_chain
        .fixture
        .init_gas_config(&gas_utils)
        .await
        .unwrap();

    let salt = keccak::hash(b"our cool token").0;
    let token_name = "MyToken";
    let token_symbol = "MTK";
    let deploy_instruction = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .decimals(18)
        .salt(salt)
        .minter(solana_chain.fixture.payer.pubkey().as_ref().to_vec())
        .gas_value(0)
        .gas_service(axelar_solana_gas_service::id())
        .gas_config_pda(gas_utils.config_pda)
        .build();

    solana_chain
        .fixture
        .send_tx(&[
            axelar_solana_its::instructions::deploy_interchain_token(deploy_instruction).unwrap(),
        ])
        .await;

    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (mint, _) = axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);
    let (metadata_account_key, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);
    let metadata_account = solana_chain
        .try_get_account_no_checks(&metadata_account_key)
        .await
        .unwrap()
        .unwrap();
    let metadata =
        mpl_token_metadata::accounts::Metadata::from_bytes(&metadata_account.data).unwrap();

    // The trailing garbage seems to be expected as fixed size buffers are used internally by
    // mpl-token-metadata: https://github.com/metaplex-foundation/mpl-token-metadata/issues/9
    assert_eq!(token_name, metadata.name.trim_end_matches('\0'));
    assert_eq!(token_symbol, metadata.symbol.trim_end_matches('\0'));
}
