use axelar_message_primitives::EncodingScheme;
use axelar_solana_memo_program_old::instruction::AxelarMemoInstruction;
use axelar_solana_memo_program_old::state::Counter;
use axelar_solana_multicall::instructions::MultiCallPayloadBuilder;
use gateway::commands::OwnedCommand;
use solana_program::instruction::AccountMeta;
use solana_program_test::tokio;
use test_fixtures::axelar_message::custom_message;

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
                axelar_solana_memo_program_old::id(),
                vec![counter_account.clone()],
                borsh::to_vec(&AxelarMemoInstruction::ProcessMemo {
                    memo: (*memo).to_string(),
                })
                .expect("failed to create multicall instruction"),
            )
            .expect("faled to create multicall instruction");
    }

    for encoding in [EncodingScheme::Borsh, EncodingScheme::AbiEncoding] {
        let payload = multicall_builder
            .clone()
            .encoding_scheme(encoding)
            .build()
            .expect("failed to build data payload");
        let message = custom_message(axelar_solana_multicall::id(), &payload);

        let (gateway_approved_command_pdas, _, _) = solana_chain
            .fixture
            .fully_approve_messages(
                &solana_chain.gateway_root_pda,
                vec![message.clone()],
                &solana_chain.signers,
                &solana_chain.domain_separator,
            )
            .await;

        let approve_message_command = OwnedCommand::ApproveMessage(message);
        let tx = solana_chain
            .fixture
            .call_execute_on_axelar_executable(
                &approve_message_command,
                &payload,
                &gateway_approved_command_pdas[0],
                &solana_chain.gateway_root_pda,
            )
            .await;

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
        .fixture
        .get_account::<Counter>(
            &memo_program_counter_pda,
            &axelar_solana_memo_program_old::ID,
        )
        .await;

    assert_eq!(counter.counter, 6);
}

#[tokio::test]
async fn test_empty_multicall_should_succeed() {
    let TestContext {
        mut solana_chain, ..
    } = axelar_solana_setup().await;

    for encoding in [EncodingScheme::Borsh, EncodingScheme::AbiEncoding] {
        let payload = MultiCallPayloadBuilder::default()
            .encoding_scheme(encoding)
            .build()
            .expect("failed to build data payload");
        let message = custom_message(axelar_solana_multicall::id(), &payload);
        let (gateway_approved_command_pdas, _, _) = solana_chain
            .fixture
            .fully_approve_messages(
                &solana_chain.gateway_root_pda,
                vec![message.clone()],
                &solana_chain.signers,
                &solana_chain.domain_separator,
            )
            .await;

        let approve_message_command = OwnedCommand::ApproveMessage(message);

        // Panics if tx fails
        let _tx = solana_chain
            .fixture
            .call_execute_on_axelar_executable(
                &approve_message_command,
                &payload,
                &gateway_approved_command_pdas[0],
                &solana_chain.gateway_root_pda,
            )
            .await;
    }
}
