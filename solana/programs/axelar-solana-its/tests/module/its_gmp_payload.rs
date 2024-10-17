#![cfg(test)]
use alloy_primitives::Bytes;
use alloy_sol_types::SolValue;
use axelar_rkyv_encoding::test_fixtures::random_message_with_destination_and_payload;
use axelar_solana_its::instructions::ItsGmpInstructionInputs;
use axelar_solana_its::state::token_manager::TokenManager;
use interchain_token_transfer_gmp::{DeployTokenManager, GMPPayload};
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::state::Mint;
use spl_token_metadata_interface::state::TokenMetadata;

use crate::program_test;

#[rstest::rstest]
#[case(spl_token::id(), Some(Pubkey::new_unique()))]
#[case(spl_token_2022::id(), Some(Pubkey::new_unique()))]
#[case(spl_token_2022::id(), None)]
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_its_gmp_payload_deploy_token_manager(
    #[case] token_program_id: Pubkey,
    #[case] operator_id: Option<Pubkey>,
) {
    use axelar_solana_its::instructions::ItsGmpInstructionInputs;

    let mut solana_chain = program_test().await;
    let (its_root_pda, its_root_pda_bump) =
        axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            &solana_chain.fixture.payer.pubkey(),
            &solana_chain.gateway_root_pda,
            &(its_root_pda, its_root_pda_bump),
        )
        .unwrap()])
        .await;

    let token_id =
        Pubkey::create_with_seed(&its_root_pda, "test_token", &axelar_solana_its::id()).unwrap();
    let operator = operator_id.map(Pubkey::to_bytes).unwrap_or_default();
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, token_id.as_ref());
    let mint_authority = axelar_solana_its::find_token_manager_pda(&interchain_token_pda).0;
    let mint = solana_chain
        .fixture
        .init_new_mint(mint_authority, token_program_id)
        .await;

    let its_gmp_payload = GMPPayload::DeployTokenManager(DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: token_id.to_bytes().into(),
        token_manager_type: alloy_primitives::Uint::<256, 4>::from(4_u128),
        params: (operator.as_ref(), mint.to_bytes()).abi_encode().into(),
    });
    let abi_payload = its_gmp_payload.encode();
    let payload_hash = solana_sdk::keccak::hash(&abi_payload).to_bytes();
    let message = random_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );
    // Action: "Relayer" calls Gateway to approve messages
    let (gateway_approved_command_pdas, _, _) = solana_chain
        .fixture
        .fully_approve_messages(
            &solana_chain.gateway_root_pda,
            vec![message.clone()],
            &solana_chain.signers,
            &solana_chain.domain_separator,
        )
        .await;

    let its_ix_inputs = ItsGmpInstructionInputs {
        payer: solana_chain.fixture.payer.pubkey(),
        gateway_approved_message_pda: gateway_approved_command_pdas[0],
        gateway_root_pda: solana_chain.gateway_root_pda,
        gmp_metadata: message.into(),
        payload: its_gmp_payload,
        token_program: token_program_id,
        mint: None,
        bumps: None,
    };

    solana_chain
        .fixture
        .send_tx_with_metadata(&[
            axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs).unwrap(),
        ])
        .await;

    let token_manager = solana_chain
        .fixture
        .get_rkyv_account::<TokenManager>(&mint_authority, &axelar_solana_its::id())
        .await;

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(mint.as_ref(), token_manager.token_address.as_ref());
}

#[rstest::rstest]
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_its_gmp_payload_deploy_interchain_token() {
    use interchain_token_transfer_gmp::DeployInterchainToken;

    let mut solana_chain = program_test().await;
    let (its_root_pda, its_root_pda_bump) =
        axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            &solana_chain.fixture.payer.pubkey(),
            &solana_chain.gateway_root_pda,
            &(its_root_pda, its_root_pda_bump),
        )
        .unwrap()])
        .await;

    let token_id =
        Pubkey::create_with_seed(&its_root_pda, "test_token", &axelar_solana_its::id()).unwrap();
    dbg!(&token_id);
    let mint = axelar_solana_its::find_interchain_token_pda(&its_root_pda, token_id.as_ref()).0;

    dbg!(&token_id);

    let deploy_interchain_token = DeployInterchainToken {
        selector: alloy_primitives::Uint::<256, 4>::from(1_u128),
        token_id: token_id.to_bytes().into(),
        name: "Test Token".to_owned(),
        symbol: "TSTTK".to_owned(),
        decimals: 8,
        minter: Bytes::new(),
    };
    let its_gmp_payload = GMPPayload::DeployInterchainToken(deploy_interchain_token.clone());
    let abi_payload = its_gmp_payload.encode();
    let payload_hash = solana_sdk::keccak::hash(&abi_payload).to_bytes();
    let message = random_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );
    // Action: "Relayer" calls Gateway to approve messages
    let (gateway_approved_command_pdas, _, _) = solana_chain
        .fixture
        .fully_approve_messages(
            &solana_chain.gateway_root_pda,
            vec![message.clone()],
            &solana_chain.signers,
            &solana_chain.domain_separator,
        )
        .await;

    let its_ix_inputs = ItsGmpInstructionInputs {
        payer: solana_chain.fixture.payer.pubkey(),
        gateway_approved_message_pda: gateway_approved_command_pdas[0],
        gateway_root_pda: solana_chain.gateway_root_pda,
        gmp_metadata: message.into(),
        payload: its_gmp_payload,
        token_program: spl_token_2022::id(),
        mint: None,
        bumps: None,
    };

    solana_chain
        .fixture
        .send_tx_with_metadata(&[
            axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs).unwrap(),
        ])
        .await;

    let mint_account = solana_chain
        .fixture
        .banks_client
        .get_account(mint)
        .await
        .expect("banks client error")
        .expect("mint account empty");

    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_account.data).unwrap();
    let token_metadata = mint_state
        .get_variable_len_extension::<TokenMetadata>()
        .unwrap();

    assert_eq!(deploy_interchain_token.name, token_metadata.name);

    let (token_manager_pda, _bump) = axelar_solana_its::find_token_manager_pda(&mint);

    let token_manager = solana_chain
        .fixture
        .get_rkyv_account::<TokenManager>(&token_manager_pda, &axelar_solana_its::id())
        .await;

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(mint.as_ref(), token_manager.token_address.as_ref());
}
