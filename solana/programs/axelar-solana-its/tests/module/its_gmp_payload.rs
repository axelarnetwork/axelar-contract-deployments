#![cfg(test)]
use alloy_primitives::Bytes;
use axelar_solana_gateway::{get_incoming_message_pda, state::incoming_message::command_id};
use axelar_solana_its::instructions::ItsGmpInstructionInputs;
use axelar_solana_its::state::token_manager::TokenManager;
use borsh::BorshDeserialize;
use interchain_token_transfer_gmp::{DeployTokenManager, GMPPayload};
use solana_program_test::tokio;
use solana_sdk::clock::Clock;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use spl_token_2022::extension::transfer_fee::TransferFeeConfig;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::state::Mint;
use spl_token_metadata_interface::state::TokenMetadata;

use crate::{
    prepare_receive_from_hub, program_test, random_hub_message_with_destination_and_payload,
    TokenUtils,
};
#[rstest::rstest]
#[case(spl_token::id(), Some(Pubkey::new_unique()))]
#[case(spl_token_2022::id(), Some(Pubkey::new_unique()))]
#[case(spl_token_2022::id(), None)]
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_its_gmp_payload_deploy_token_manager(
    #[case] token_program_id: Pubkey,
    #[case] operator: Option<Pubkey>,
) {
    let mut solana_chain = program_test().await;
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;

    let token_id = Pubkey::create_with_seed(&its_root_pda, "test_token", &axelar_solana_its::id())
        .unwrap()
        .to_bytes();
    let (mint_authority, _) = axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let mint = solana_chain
        .fixture
        .init_new_mint(mint_authority, token_program_id, 18)
        .await;

    let inner_payload = GMPPayload::DeployTokenManager(DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: token_id.into(),
        token_manager_type: alloy_primitives::Uint::<256, 4>::from(4_u128),
        params: axelar_solana_its::state::token_manager::encode_params(
            operator,
            Some(solana_chain.fixture.payer.pubkey()),
            mint,
        )
        .into(),
    });

    let its_gmp_payload = prepare_receive_from_hub(&inner_payload, "ethereum".to_owned());
    let abi_payload = its_gmp_payload.encode();
    let payload_hash = solana_sdk::keccak::hash(&abi_payload).to_bytes();
    let message = random_hub_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );

    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) =
        get_incoming_message_pda(&command_id(&message.cc_id.chain, &message.cc_id.id));

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message(merkelised_message.leaf.message)
        .payload(its_gmp_payload)
        .token_program(token_program_id)
        .build();

    let instruction = axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs)
        .expect("failed to create instruction");

    solana_chain.fixture.send_tx(&[instruction]).await.unwrap();

    let data = solana_chain
        .fixture
        .get_account(&mint_authority, &axelar_solana_its::id())
        .await
        .data;
    let token_manager = TokenManager::try_from_slice(&data).unwrap();

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(mint.as_ref(), token_manager.token_address.as_ref());
}

#[rstest::rstest]
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_its_gmp_payload_deploy_interchain_token() {
    use interchain_token_transfer_gmp::DeployInterchainToken;

    let mut solana_chain = program_test().await;
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;

    let token_id = Pubkey::create_with_seed(&its_root_pda, "test_token", &axelar_solana_its::id())
        .unwrap()
        .to_bytes();
    let mint = axelar_solana_its::find_interchain_token_pda(&its_root_pda, token_id.as_ref()).0;
    let deploy_interchain_token = DeployInterchainToken {
        selector: alloy_primitives::Uint::<256, 4>::from(1_u128),
        token_id: token_id.into(),
        name: "Test Token".to_owned(),
        symbol: "TSTTK".to_owned(),
        decimals: 8,
        minter: Bytes::new(),
    };
    let inner_payload = GMPPayload::DeployInterchainToken(deploy_interchain_token.clone());
    let its_gmp_payload = prepare_receive_from_hub(&inner_payload, "ethereum".to_owned());
    let abi_payload = its_gmp_payload.encode();
    let payload_hash = solana_sdk::keccak::hash(&abi_payload).to_bytes();
    let message = random_hub_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );
    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) =
        get_incoming_message_pda(&command_id(&message.cc_id.chain, &message.cc_id.id));

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message(merkelised_message.leaf.message)
        .payload(its_gmp_payload)
        .token_program(spl_token_2022::id())
        .build();

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs).unwrap()])
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

    let (token_manager_pda, _bump) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);

    let data = solana_chain
        .fixture
        .get_account(&token_manager_pda, &axelar_solana_its::id())
        .await
        .data;
    let token_manager = TokenManager::try_from_slice(&data).unwrap();

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(mint.as_ref(), token_manager.token_address.as_ref());
}

#[rstest::rstest]
#[case(spl_token::id())]
#[case(spl_token_2022::id())]
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_its_gmp_payload_interchain_transfer_lock_unlock(#[case] token_program_id: Pubkey) {
    use axelar_solana_its::instructions::ItsGmpInstructionInputs;
    use axelar_solana_its::state::token_manager;
    use interchain_token_transfer_gmp::InterchainTransfer;

    let mut solana_chain = program_test().await;
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;

    let token_id = Pubkey::create_with_seed(&its_root_pda, "test_token", &axelar_solana_its::id())
        .unwrap()
        .to_bytes();
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let mint = solana_chain
        .fixture
        .init_new_mint(solana_chain.fixture.payer.pubkey(), token_program_id, 18)
        .await;

    let inner_payload = GMPPayload::DeployTokenManager(DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: token_id.into(),
        token_manager_type: token_manager::Type::LockUnlock.into(),
        params: axelar_solana_its::state::token_manager::encode_params(None, None, mint).into(),
    });

    let its_gmp_payload = prepare_receive_from_hub(&inner_payload, "ethereum".to_owned());
    let abi_payload = its_gmp_payload.encode();
    let payload_hash = solana_sdk::keccak::hash(&abi_payload).to_bytes();
    let message = random_hub_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );
    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) =
        get_incoming_message_pda(&command_id(&message.cc_id.chain, &message.cc_id.id));

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message(merkelised_message.leaf.message)
        .payload(its_gmp_payload)
        .token_program(token_program_id)
        .build();

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs).unwrap()])
        .await;

    let data = solana_chain
        .fixture
        .get_account(&token_manager_pda, &axelar_solana_its::id())
        .await
        .data;
    let token_manager = TokenManager::try_from_slice(&data).unwrap();

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(mint.as_ref(), token_manager.token_address.as_ref());

    let token_manager_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &token_manager_pda,
            &mint,
            &token_program_id,
        );

    // Fund the token manager to simulate locked tokens.
    let locked_amount = 5000_u64;
    solana_chain
        .fixture
        .mint_tokens_to(
            mint,
            token_manager_ata,
            solana_chain.fixture.payer.insecure_clone(),
            locked_amount,
            token_program_id,
        )
        .await;

    let transferred_amount = 1234_u64;
    let inner_transfer_payload = GMPPayload::InterchainTransfer(InterchainTransfer {
        selector: alloy_primitives::Uint::<256, 4>::from(0_u128),
        token_id: token_id.into(),
        source_address: token_id.into(), // Does't matter
        destination_address: solana_chain.fixture.payer.pubkey().to_bytes().into(),
        amount: alloy_primitives::Uint::<256, 4>::from(transferred_amount),
        data: Bytes::new(),
    });

    let its_gmp_transfer_payload =
        prepare_receive_from_hub(&inner_transfer_payload, "ethereum".to_owned());
    let transfer_abi_payload = its_gmp_transfer_payload.encode();
    let transfer_payload_hash = solana_sdk::keccak::hash(&transfer_abi_payload).to_bytes();
    let message = random_hub_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        transfer_payload_hash,
    );
    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) =
        get_incoming_message_pda(&command_id(&message.cc_id.chain, &message.cc_id.id));

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let clock_sysvar = solana_chain
        .fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap();

    let transfer_its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message(merkelised_message.leaf.message)
        .payload(its_gmp_transfer_payload)
        .token_program(token_program_id)
        .timestamp(clock_sysvar.unix_timestamp)
        .mint(mint)
        .build();

    solana_chain
        .fixture
        .send_tx(&[
            axelar_solana_its::instructions::its_gmp_payload(transfer_its_ix_inputs).unwrap(),
        ])
        .await;

    let token_manager_ata_account = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(token_manager_ata)
        .await
        .unwrap();

    let destination_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &solana_chain.fixture.payer.pubkey(),
            &mint,
            &token_program_id,
        );

    let destination_ata_account = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(destination_ata)
        .await
        .unwrap();

    assert_eq!(
        token_manager_ata_account.amount,
        locked_amount - transferred_amount,
        "New balance doesn't match expected balance"
    );

    assert_eq!(
        destination_ata_account.amount, transferred_amount,
        "New balance doesn't match expected balance"
    );
}

#[rstest::rstest]
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_its_gmp_payload_interchain_transfer_lock_unlock_fee() {
    use axelar_solana_its::instructions::ItsGmpInstructionInputs;
    use axelar_solana_its::state::token_manager;
    use interchain_token_transfer_gmp::InterchainTransfer;

    let mut solana_chain = program_test().await;
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;

    let token_id = Pubkey::create_with_seed(&its_root_pda, "test_token", &axelar_solana_its::id())
        .unwrap()
        .to_bytes();
    let fee_basis_points = 50_u16;
    let maximum_fee = u64::MAX;
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let mint = solana_chain
        .fixture
        .init_new_mint_with_fee(
            solana_chain.fixture.payer.pubkey(),
            spl_token_2022::id(),
            fee_basis_points,
            maximum_fee,
            0,
            None,
            None,
        )
        .await;

    let inner_payload = GMPPayload::DeployTokenManager(DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: token_id.into(),
        token_manager_type: token_manager::Type::LockUnlockFee.into(),
        params: axelar_solana_its::state::token_manager::encode_params(None, None, mint).into(),
    });

    let its_gmp_payload = prepare_receive_from_hub(&inner_payload, "ethereum".to_owned());
    let abi_payload = its_gmp_payload.encode();
    let payload_hash = solana_sdk::keccak::hash(&abi_payload).to_bytes();
    let message = random_hub_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );
    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) =
        get_incoming_message_pda(&command_id(&message.cc_id.chain, &message.cc_id.id));

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message(merkelised_message.leaf.message)
        .payload(its_gmp_payload)
        .token_program(spl_token_2022::id())
        .build();

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs).unwrap()])
        .await;

    let data = solana_chain
        .fixture
        .get_account(&token_manager_pda, &axelar_solana_its::id())
        .await
        .data;
    let token_manager = TokenManager::try_from_slice(&data).unwrap();

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(mint.as_ref(), token_manager.token_address.as_ref());

    let token_manager_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &token_manager_pda,
            &mint,
            &spl_token_2022::id(),
        );

    // Fund the token manager to simulate locked tokens.
    let locked_amount = 5000_u64;
    solana_chain
        .fixture
        .mint_tokens_to(
            mint,
            token_manager_ata,
            solana_chain.fixture.payer.insecure_clone(),
            locked_amount,
            spl_token_2022::id(),
        )
        .await;

    let transferred_amount = 1234_u64;
    let inner_transfer_payload = GMPPayload::InterchainTransfer(InterchainTransfer {
        selector: alloy_primitives::Uint::<256, 4>::from(0_u128),
        token_id: token_id.into(),
        source_address: token_id.into(), // Does't matter
        destination_address: solana_chain.fixture.payer.pubkey().to_bytes().into(),
        amount: alloy_primitives::Uint::<256, 4>::from(transferred_amount),
        data: Bytes::new(),
    });

    let its_gmp_transfer_payload =
        prepare_receive_from_hub(&inner_transfer_payload, "ethereum".to_owned());
    let transfer_abi_payload = its_gmp_transfer_payload.encode();
    let transfer_payload_hash = solana_sdk::keccak::hash(&transfer_abi_payload).to_bytes();
    let message = random_hub_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        transfer_payload_hash,
    );
    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) =
        get_incoming_message_pda(&command_id(&message.cc_id.chain, &message.cc_id.id));

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let clock_sysvar = solana_chain
        .fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap();
    let transfer_its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message(merkelised_message.leaf.message)
        .payload(its_gmp_transfer_payload)
        .token_program(spl_token_2022::id())
        .timestamp(clock_sysvar.unix_timestamp)
        .mint(mint)
        .build();

    solana_chain
        .fixture
        .send_tx(&[
            axelar_solana_its::instructions::its_gmp_payload(transfer_its_ix_inputs).unwrap(),
        ])
        .await;

    let token_manager_ata_account = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(token_manager_ata)
        .await
        .unwrap();

    let destination_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &solana_chain.fixture.payer.pubkey(),
            &mint,
            &spl_token_2022::id(),
        );

    let destination_ata_account = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(destination_ata)
        .await
        .unwrap();

    assert_eq!(
        token_manager_ata_account.amount,
        locked_amount - transferred_amount,
        "New balance doesn't match expected balance"
    );

    let mint_data = solana_chain
        .fixture
        .banks_client
        .get_account(mint)
        .await
        .unwrap()
        .unwrap();

    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data.data).unwrap();
    let fee_config = mint_state.get_extension::<TransferFeeConfig>().unwrap();
    let epoch = solana_chain
        .fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap()
        .epoch;
    let fee = fee_config
        .calculate_epoch_fee(epoch, transferred_amount)
        .unwrap();

    assert_eq!(
        destination_ata_account.amount,
        transferred_amount.checked_sub(fee).unwrap(),
        "New balance doesn't match expected balance"
    );
}
