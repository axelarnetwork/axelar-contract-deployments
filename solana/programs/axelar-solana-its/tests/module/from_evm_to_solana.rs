use axelar_rkyv_encoding::test_fixtures::random_message_with_destination_and_payload;
use axelar_solana_its::instructions::ItsGmpInstructionInputs;
use axelar_solana_its::state::token_manager::TokenManager;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_gateway::ContractCallFilter;
use interchain_token_transfer_gmp::GMPPayload;
use solana_program_test::tokio;
use solana_sdk::signer::Signer;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::state::Mint;
use spl_token_metadata_interface::state::TokenMetadata;

use crate::{axelar_evm_setup, axelar_solana_setup, ItsProgramWrapper};

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_send_from_evm_to_solana() {
    // Setup - Solana
    let ItsProgramWrapper {
        mut solana_chain,
        chain_name: solana_chain_name,
        ..
    } = axelar_solana_setup().await;
    // Setup - EVM
    let (_evm_chain, evm_signer, its_contracts) = axelar_evm_setup().await;
    let token_name = "Canonical Token";
    let token_symbol = "CT";
    let test_its_canonical_token = evm_signer
        .deploy_axelar_test_canonical_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await
        .unwrap();
    its_contracts
        .interchain_token_factory
        .register_canonical_interchain_token(test_its_canonical_token.address())
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    let event_filter = its_contracts
        .interchain_token_service
        .interchain_token_id_claimed_filter();
    let token_id = event_filter
        .query()
        .await
        .unwrap()
        .first()
        .unwrap()
        .token_id;
    its_contracts
        .interchain_token_factory
        .deploy_remote_canonical_interchain_token(
            String::new(),
            test_its_canonical_token.address(),
            solana_chain_name.clone(),
            0_u128.into(),
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    let log: ContractCallFilter = its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("no logs found");

    let payload = log.payload.as_ref().to_vec();
    let payload_hash = solana_sdk::keccak::hash(&payload).to_bytes();
    let axelar_message = random_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );

    // - The relayer relays the message to the Solana gateway
    let (gateway_approved_command_pdas, _, _) = solana_chain
        .fixture
        .fully_approve_messages(
            &solana_chain.gateway_root_pda,
            vec![axelar_message.clone()],
            &solana_chain.signers,
            &solana_chain.domain_separator,
        )
        .await;

    let its_ix_inputs = ItsGmpInstructionInputs {
        payer: solana_chain.fixture.payer.pubkey(),
        gateway_approved_message_pda: gateway_approved_command_pdas[0],
        gateway_root_pda: solana_chain.gateway_root_pda,
        gmp_metadata: axelar_message.into(),
        payload: GMPPayload::decode(&payload).unwrap(),
        token_program: spl_token_2022::id(),
        mint: None,
        bumps: None,
    };

    // - Relayer calls the Solana ITS program
    let instruction = axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs)
        .expect("failed to create instruction");

    let _tx1 = solana_chain
        .fixture
        .send_tx_with_metadata(&[instruction])
        .await;
    let (its_root_pda, _its_root_pda_bump) =
        axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (mint, _) = axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

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

    assert_eq!(token_name, token_metadata.name);
    assert_eq!(token_symbol, token_metadata.symbol);

    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, token_id.as_ref());
    let (token_manager_pda, _bump) =
        axelar_solana_its::find_token_manager_pda(&interchain_token_pda);

    let token_manager = solana_chain
        .fixture
        .get_rkyv_account::<TokenManager>(&token_manager_pda, &axelar_solana_its::id())
        .await;

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(mint.as_ref(), token_manager.token_address.as_ref());
}
