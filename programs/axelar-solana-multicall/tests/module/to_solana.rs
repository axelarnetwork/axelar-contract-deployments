use axelar_executable::EncodingScheme;
use axelar_solana_gateway_test_fixtures::gateway::random_message;
use axelar_solana_memo_program::instruction::AxelarMemoInstruction;
use axelar_solana_memo_program::state::Counter;
use axelar_solana_multicall::instructions::MultiCallPayloadBuilder;
use borsh::BorshDeserialize as _;
use solana_program::instruction::AccountMeta;
use solana_program_test::tokio;

use crate::{axelar_solana_setup, TestContext};

#[tokio::test]
async fn test_multicall_different_encodings() {
    let TestContext {
        mut solana_chain,
        memo_program_counter_pda,
    } = axelar_solana_setup().await;

    let counter_account = AccountMeta {
        pubkey: memo_program_counter_pda,
        is_signer: false,
        is_writable: true,
    };
    let mut multicall_builder = MultiCallPayloadBuilder::default();

    for memo in &["Call A", "Call B", "Call C"] {
        multicall_builder = multicall_builder
            .add_instruction(
                axelar_solana_memo_program::id(),
                vec![counter_account.clone()],
                borsh::to_vec(&AxelarMemoInstruction::ProcessMemo {
                    memo: (*memo).to_string(),
                })
                .expect("failed to create multicall instruction"),
            )
            .expect("faled to create multicall instruction");
    }

    for encoding in [EncodingScheme::Borsh, EncodingScheme::AbiEncoding] {
        let mut builder = multicall_builder.clone().encoding_scheme(encoding);
        let payload = builder.build().expect("failed to build data payload");
        let mut message = random_message();
        message.destination_address = axelar_solana_multicall::id().to_string();
        message.payload_hash = *payload.hash().unwrap();

        let message_from_multisig_prover = solana_chain
            .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
            .await
            .unwrap();

        let merkelised_message = message_from_multisig_prover
            .iter()
            .find(|x| x.leaf.message.cc_id == message.cc_id)
            .unwrap()
            .clone();

        let tx = solana_chain
            .execute_on_axelar_executable(
                merkelised_message.leaf.message,
                &payload.encode().unwrap(),
            )
            .await
            .unwrap();

        let log_msgs = tx.metadata.unwrap().log_messages;
        assert!(
            log_msgs.iter().any(|log| log.as_str().contains("Call A")),
            "expected memo not found in logs"
        );

        assert!(
            log_msgs.iter().any(|log| log.as_str().contains("Call B")),
            "expected memo not found in logs"
        );

        assert!(
            log_msgs.iter().any(|log| log.as_str().contains("Call C")),
            "expected memo not found in logs"
        );
    }

    let counter = solana_chain
        .get_account(&memo_program_counter_pda, &axelar_solana_memo_program::ID)
        .await;
    let counter = Counter::try_from_slice(&counter.data).unwrap();
    assert_eq!(counter.counter, 6);
}

#[tokio::test]
async fn test_empty_multicall_should_succeed() {
    let TestContext {
        mut solana_chain, ..
    } = axelar_solana_setup().await;

    for encoding in [EncodingScheme::Borsh, EncodingScheme::AbiEncoding] {
        let mut builder = MultiCallPayloadBuilder::default().encoding_scheme(encoding);
        let payload = builder.build().expect("failed to build data payload");
        let mut message = random_message();
        message.destination_address = axelar_solana_multicall::id().to_string();
        message.payload_hash = *payload.hash().unwrap();
        let message_from_multisig_prover = solana_chain
            .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
            .await
            .unwrap();

        let merkelised_message = message_from_multisig_prover
            .iter()
            .find(|x| x.leaf.message.cc_id == message.cc_id)
            .unwrap()
            .clone();

        let _tx = solana_chain
            .execute_on_axelar_executable(
                merkelised_message.leaf.message,
                &payload.encode().unwrap(),
            )
            .await
            .unwrap();
    }
}
