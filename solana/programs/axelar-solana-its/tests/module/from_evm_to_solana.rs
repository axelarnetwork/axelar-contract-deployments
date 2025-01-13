use axelar_executable::{AxelarMessagePayload, EncodingScheme, SolanaAccountRepr};
use axelar_solana_its::state::token_manager::TokenManager;
use axelar_solana_memo_program::state::Counter;
use borsh::BorshDeserialize;
use evm_contracts_test_suite::ethers::signers::Signer;
use evm_contracts_test_suite::ethers::types::{Address, Bytes};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;
use evm_contracts_test_suite::ItsContracts;
use interchain_token_transfer_gmp::GMPPayload;
use solana_program_test::tokio;
use solana_sdk::program_pack::Pack as _;

use crate::{axelar_evm_setup, axelar_solana_setup, relay_to_solana, ItsProgramWrapper};

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

    let payload = if let Ok(GMPPayload::SendToHub(inner)) = GMPPayload::decode(&payload) {
        inner.payload.to_vec()
    } else {
        panic!("unexpected payload type")
    };
    relay_to_solana(payload, &mut solana_chain, None, spl_token_2022::id()).await;

    let (its_root_pda, _its_root_pda_bump) =
        axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (mint, _) = axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    let (metadata_account_key, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);
    let metadata_account = solana_chain
        .try_get_account_no_checks(&metadata_account_key)
        .await
        .unwrap()
        .unwrap();
    let token_metadata =
        mpl_token_metadata::accounts::Metadata::from_bytes(&metadata_account.data).unwrap();

    assert_eq!(token_name, token_metadata.name.trim_end_matches('\0'));
    assert_eq!(token_symbol, token_metadata.symbol.trim_end_matches('\0'));

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
            &AxelarMessagePayload::new(
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
    let payload = if let Ok(GMPPayload::SendToHub(inner)) = GMPPayload::decode(&transfer_payload) {
        inner.payload.to_vec()
    } else {
        panic!("unexpected payload type");
    };
    let tx = relay_to_solana(payload, &mut solana_chain, Some(mint), spl_token_2022::id()).await;

    let ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &axelar_solana_memo_program::id(),
        &mint,
        &spl_token_2022::id(),
    );

    let ata_raw_account = solana_chain.try_get_account_no_checks(&ata).await.unwrap();
    let ata_account =
        spl_token_2022::state::Account::unpack_from_slice(&ata_raw_account.unwrap().data).unwrap();

    assert_eq!(ata_account.mint, mint);
    assert_eq!(ata_account.owner, axelar_solana_memo_program::id());
    assert_eq!(ata_account.amount, transfer_amount);

    let log_msgs = tx.metadata.unwrap().log_messages;
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("🐪🐪🐪🐪")),
        "expected memo not found in logs"
    );
    let counter_raw_account = solana_chain
        .try_get_account_no_checks(&counter_pda.unwrap())
        .await
        .unwrap()
        .unwrap();
    let counter = Counter::try_from_slice(&counter_raw_account.data).unwrap();

    assert_eq!(counter.counter, 1);
}
