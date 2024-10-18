use axelar_rkyv_encoding::test_fixtures::random_message_with_destination_and_payload;
use axelar_solana_its::instructions::ItsGmpInstructionInputs;
use axelar_solana_its::state::token_manager::TokenManager;
use evm_contracts_test_suite::ethers::signers::Signer;
use evm_contracts_test_suite::ethers::types::{Address, Bytes};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_gateway::ContractCallFilter;
use evm_contracts_test_suite::ItsContracts;
use interchain_token_transfer_gmp::GMPPayload;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer as _;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::state::Mint;
use spl_token_metadata_interface::state::TokenMetadata;
use test_fixtures::test_setup::SolanaAxelarIntegrationMetadata;

use crate::{axelar_evm_setup, axelar_solana_setup, ItsProgramWrapper};

async fn setup_canonical_interchain_token(
    its_contracts: &ItsContracts,
    solana_chain_name: String,
    token_address: Address,
) -> Result<([u8; 32], Vec<u8>), Box<dyn std::error::Error>> {
    its_contracts
        .interchain_token_factory
        .register_canonical_interchain_token(token_address)
        .send()
        .await?
        .await?
        .ok_or("failed to register canonical interchain token")?;

    let event_filter = its_contracts
        .interchain_token_service
        .interchain_token_id_claimed_filter();

    let token_id = event_filter
        .query()
        .await?
        .first()
        .ok_or("no token id found")?
        .token_id;

    its_contracts
        .interchain_token_factory
        .deploy_remote_canonical_interchain_token(
            String::new(),
            token_address,
            solana_chain_name,
            0_u128.into(),
        )
        .send()
        .await?
        .await?
        .ok_or("failed to deploy remote canonical interchain token")?;

    let log: ContractCallFilter = its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await?
        .into_iter()
        .next()
        .ok_or("no logs found")?;

    Ok((token_id, log.payload.as_ref().to_vec()))
}

async fn relay_to_solana(
    payload: Vec<u8>,
    solana_chain: &mut SolanaAxelarIntegrationMetadata,
    maybe_mint: Option<Pubkey>,
) {
    let payload_hash = solana_sdk::keccak::hash(&payload).to_bytes();
    let axelar_message = random_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );

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
        mint: maybe_mint,
        bumps: None,
    };

    let instruction = axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs)
        .expect("failed to create instruction");
    let _tx = solana_chain
        .fixture
        .send_tx_with_metadata(&[instruction])
        .await;
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_send_from_evm_to_solana() {
    let ItsProgramWrapper {
        mut solana_chain,
        chain_name: solana_chain_name,
    } = axelar_solana_setup().await;
    let (_evm_chain, evm_signer, its_contracts) = axelar_evm_setup().await;

    let token_name = "Canonical Token";
    let token_symbol = "CT";
    let test_its_canonical_token = evm_signer
        .deploy_axelar_test_canonical_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await
        .unwrap();

    let (token_id, payload) = setup_canonical_interchain_token(
        &its_contracts,
        solana_chain_name.clone(),
        test_its_canonical_token.address(),
    )
    .await
    .expect("failed to setup interchain token from canonical token");

    relay_to_solana(payload, &mut solana_chain, None).await;

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

    let _receipt = test_its_canonical_token
        .mint(evm_signer.wallet.address(), u64::MAX.into())
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    test_its_canonical_token
        .approve(
            its_contracts.interchain_token_service.address(),
            u64::MAX.into(),
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    let transfer_amount = 500_000_u64;
    its_contracts
        .interchain_token_service
        .interchain_transfer(
            token_id,
            solana_chain_name.clone(),
            solana_chain.fixture.payer.pubkey().as_ref().to_vec().into(),
            transfer_amount.into(),
            Bytes::new(),
            0_u128.into(),
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    let transfer_log: ContractCallFilter = its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("no logs found");

    let transfer_payload = transfer_log.payload.as_ref().to_vec();
    relay_to_solana(transfer_payload, &mut solana_chain, Some(mint)).await;

    let ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &solana_chain.fixture.payer.pubkey(),
        &mint,
        &spl_token_2022::id(),
    );

    let ata_account = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(ata)
        .await
        .unwrap();

    assert_eq!(ata_account.mint, mint);
    assert_eq!(ata_account.owner, solana_chain.fixture.payer.pubkey());
    assert_eq!(ata_account.amount, transfer_amount);
}
