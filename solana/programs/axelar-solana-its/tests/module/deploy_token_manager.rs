#![cfg(test)]
use axelar_solana_its::instructions::DeployTokenManagerInputs;
use axelar_solana_its::state::token_manager::{self, TokenManager};
use borsh::BorshDeserialize;
use solana_program_test::tokio;
use solana_sdk::keccak;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

use crate::{axelar_solana_setup, ItsProgramWrapper, TokenUtils};

#[rstest::rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
#[tokio::test]
async fn test_deploy_token_manager(#[case] token_program_id: Pubkey) {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;

    let mint = solana_chain
        .fixture
        .init_new_mint(solana_chain.fixture.payer.pubkey(), token_program_id, 18)
        .await;

    let params =
        token_manager::encode_params(None, Some(solana_chain.fixture.payer.pubkey()), mint);
    let salt = keccak::hash(b"our cool token").0;

    let deploy_instruction = DeployTokenManagerInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .salt(salt)
        .token_manager_type(token_manager::Type::LockUnlock)
        .gas_value(0)
        .params(params)
        .token_program(token_program_id)
        .build();

    solana_chain
        .fixture
        .send_tx(&[
            axelar_solana_its::instructions::deploy_token_manager(deploy_instruction).unwrap(),
        ])
        .await;

    let token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        salt.as_slice(),
    );
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);

    let data = solana_chain
        .fixture
        .get_account(&token_manager_pda, &axelar_solana_its::id())
        .await
        .data;
    let token_manager = TokenManager::deserialize(&mut data.as_ref()).unwrap();

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(mint.as_ref(), token_manager.token_address.as_ref());
}
