use axelar_message_primitives::{DataPayload, EncodingScheme, SolanaAccountRepr};
use axelar_solana_gateway::get_incoming_message_pda;
use axelar_solana_gateway::state::incoming_message::command_id;
use axelar_solana_gateway_test_fixtures::SolanaAxelarIntegrationMetadata;
use axelar_solana_its::instructions::ItsGmpInstructionInputs;
use axelar_solana_its::state::token_manager::TokenManager;
use axelar_solana_memo_program::state::Counter;
use borsh::BorshDeserialize;
use evm_contracts_test_suite::ethers::signers::Signer;
use evm_contracts_test_suite::ethers::types::{Address, Bytes};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;
use evm_contracts_test_suite::ItsContracts;
use interchain_token_transfer_gmp::{GMPPayload, ReceiveFromHub};
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::clock::Clock;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer as _;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::state::Mint;
use spl_token_metadata_interface::state::TokenMetadata;

use crate::{
    axelar_evm_setup, axelar_solana_setup, random_hub_message_with_destination_and_payload,
    ItsProgramWrapper,
};

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
        .deploy_remote_canonical_interchain_token(token_address, solana_chain_name, 0_u128.into())
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

#[allow(clippy::panic)]
async fn relay_to_solana(
    payload: Vec<u8>,
    solana_chain: &mut SolanaAxelarIntegrationMetadata,
    maybe_mint: Option<Pubkey>,
) -> BanksTransactionResultWithMetadata {
    let GMPPayload::SendToHub(inner) = GMPPayload::decode(&payload).unwrap() else {
        panic!("unexpected payload type");
    };

    let inner_payload = inner.payload;
    let solana_payload = GMPPayload::ReceiveFromHub(ReceiveFromHub {
        selector: ReceiveFromHub::MESSAGE_TYPE_ID.try_into().unwrap(),
        source_chain: "ethereum".to_owned(),
        payload: inner_payload,
    });
    let encoded_payload = solana_payload.encode();

    let payload_hash = solana_sdk::keccak::hash(&encoded_payload).to_bytes();
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

    let clock_sysvar: Clock = solana_chain
        .fixture
        .banks_client
        .get_sysvar()
        .await
        .unwrap();

    let its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message(merkelised_message.leaf.message)
        .payload(solana_payload)
        .token_program(spl_token_2022::id())
        .timestamp(clock_sysvar.unix_timestamp)
        .mint_opt(maybe_mint)
        .build();

    let instruction = axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs)
        .expect("failed to create instruction");

    solana_chain.fixture.send_tx(&[instruction]).await.unwrap()
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
#[allow(clippy::non_ascii_literal)]
#[allow(clippy::little_endian_bytes)]
async fn test_send_from_evm_to_solana() {
    let ItsProgramWrapper {
        mut solana_chain,
        chain_name: solana_chain_name,
        counter_pda,
    } = axelar_solana_setup(true).await;
    let (_evm_chain, evm_signer, its_contracts, _weighted_signers, _domain_separator) =
        axelar_evm_setup().await;

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

    let memo_instruction =
        axelar_solana_memo_program::instruction::AxelarMemoInstruction::ProcessMemo {
            memo: "🐪🐪🐪🐪".to_owned(),
        };
    let transfer_amount = 500_000_u64;
    let metadata = Bytes::from(
        [
            0_u32.to_le_bytes().as_slice(), // MetadataVersion.CONTRACT_CALL
            &DataPayload::new(
                &borsh::to_vec(&memo_instruction).unwrap(),
                &[SolanaAccountRepr {
                    pubkey: counter_pda.unwrap().to_bytes().into(),
                    is_signer: false,
                    is_writable: true,
                }],
                EncodingScheme::AbiEncoding,
            )
            .encode()
            .unwrap(),
        ]
        .concat(),
    );

    its_contracts
        .interchain_token_service
        .interchain_transfer(
            token_id,
            solana_chain_name.clone(),
            axelar_solana_memo_program::id().to_bytes().into(),
            transfer_amount.into(),
            metadata,
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
    let tx = relay_to_solana(transfer_payload, &mut solana_chain, Some(mint)).await;

    let ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &axelar_solana_memo_program::id(),
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
    assert_eq!(ata_account.owner, axelar_solana_memo_program::id());
    assert_eq!(ata_account.amount, transfer_amount);

    let log_msgs = tx.metadata.unwrap().log_messages;
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("🐪🐪🐪🐪")),
        "expected memo not found in logs"
    );
    let counter = solana_chain
        .fixture
        .banks_client
        .get_account_data_with_borsh::<Counter>(counter_pda.unwrap())
        .await
        .unwrap();

    assert_eq!(counter.counter, 1);
}
