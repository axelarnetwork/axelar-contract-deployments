use axelar_solana_its::instructions::DeployInterchainTokenInputs;
use solana_program_test::tokio;
use solana_sdk::keccak;
use solana_sdk::signer::Signer;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::state::Mint;
use spl_token_metadata_interface::state::TokenMetadata;

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

    let mint_account = solana_chain
        .try_get_account_no_checks(&mint)
        .await
        .expect("banks client error")
        .expect("mint account empty");

    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_account.data).unwrap();
    let token_metadata = mint_state
        .get_variable_len_extension::<TokenMetadata>()
        .unwrap();

    assert_eq!(token_name, token_metadata.name);
    assert_eq!(token_symbol, token_metadata.symbol);
}
