use std::time::SystemTime;

use axelar_executable_old::axelar_message_primitives::DestinationProgramId;
use axelar_executable_old::AxelarCallableInstruction;
use axelar_rkyv_encoding::types::{CrossChainId, GmpMetadata, Message};
use axelar_solana_governance::events::{EventContainer, GovernanceEvent};
use axelar_solana_governance::instructions::builder::{self, IxBuilder};
use axelar_solana_governance::state::GovernanceConfig;
use axelar_solana_memo_program_old::instruction::AxelarMemoInstruction;
use borsh::{to_vec, BorshSerialize};
use gateway::hasher_impl;
use solana_program_test::{processor, BanksTransactionResultWithMetadata, ProgramTest};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use test_fixtures::test_setup::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata, TestFixture,
};

use crate::fixtures::{
    self, operator_keypair, MINIMUM_PROPOSAL_DELAY, SOURCE_CHAIN_ADDRESS,
    SOURCE_CHAIN_ADDRESS_KECCAK_HASH, SOURCE_CHAIN_NAME, SOURCE_CHAIN_NAME_KECCAK_HASH,
};

pub(crate) async fn setup_programs() -> (SolanaAxelarIntegrationMetadata, Pubkey, Pubkey) {
    let mut fixture = TestFixture::new(program_test()).await;

    // Setup gov module (initialize contract)
    let (gov_config_pda, _) =
        init_contract_with_operator(&mut fixture, operator_keypair().pubkey().to_bytes())
            .await
            .unwrap();

    // Setup gateway
    let mut sol_integration = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![555, 222])
        .programs_to_deploy(vec![(
            "axelar_solana_memo_program_old.so".into(),
            axelar_solana_memo_program_old::id(),
        )])
        .build()
        .setup_with_fixture_and_authority(fixture, gov_config_pda)
        .await;

    // Init the memo program
    let memo_counter_pda =
        axelar_solana_memo_program_old::get_counter_pda(&sol_integration.gateway_root_pda);
    let ix = axelar_solana_memo_program_old::instruction::initialize(
        &sol_integration.fixture.payer.pubkey(),
        &sol_integration.gateway_root_pda,
        &memo_counter_pda,
    )
    .unwrap();

    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    (sol_integration, gov_config_pda, memo_counter_pda.0)
}

pub(crate) fn program_test() -> ProgramTest {
    ProgramTest::new(
        "axelar_solana_governance",
        axelar_solana_governance::id(),
        processor!(axelar_solana_governance::processor::Processor::process_instruction),
    )
}

pub(crate) async fn init_contract_with_operator(
    fixture: &mut TestFixture,
    operator: [u8; 32],
) -> Result<(Pubkey, u8), ProgramError> {
    let (config_pda, bump) = GovernanceConfig::pda();

    let config = axelar_solana_governance::state::GovernanceConfig::new(
        bump,
        SOURCE_CHAIN_NAME_KECCAK_HASH,
        SOURCE_CHAIN_ADDRESS_KECCAK_HASH,
        MINIMUM_PROPOSAL_DELAY,
        operator,
    );

    let ix = IxBuilder::new()
        .initialize_config(&fixture.payer.pubkey(), &config_pda, config.clone())
        .build();
    fixture.send_tx(&[ix]).await;
    Ok((config_pda, bump))
}

pub(crate) fn default_proposal_eta() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + u64::from(MINIMUM_PROPOSAL_DELAY)
        + 10
}

pub(crate) fn gmp_sample_metadata() -> GmpMetadata {
    GmpMetadata {
        cross_chain_id: CrossChainId::new(
            SOURCE_CHAIN_NAME.to_string(),
            uuid::Uuid::new_v4().to_string(),
        ),
        source_address: SOURCE_CHAIN_ADDRESS.to_string(),
        destination_address: axelar_solana_governance::ID.to_string(),
        destination_chain: "solana".to_string(),
        domain_separator: [0_u8; 32],
    }
}

pub(crate) fn ix_builder_with_sample_proposal_data(
) -> IxBuilder<axelar_solana_governance::instructions::builder::ProposalRelated> {
    IxBuilder::new().with_proposal_data(
        Pubkey::new_from_array(fixtures::PROPOSAL_TARGET_ADDRESS),
        1,
        default_proposal_eta(),
        Some(AccountMeta::new_readonly(
            Pubkey::new_from_array([0_u8; 32]),
            false,
        )),
        &[AccountMeta::new_readonly(
            Pubkey::new_from_array([0_u8; 32]),
            false,
        )],
        vec![0],
    )
}

pub(crate) fn ix_builder_with_memo_proposal_data(
    solana_accounts: &[AccountMeta],
    native_value: u64,
    native_target_value_account: Option<AccountMeta>,
) -> IxBuilder<axelar_solana_governance::instructions::builder::ProposalRelated> {
    let memo_instruction = AxelarCallableInstruction::Native(
        to_vec(&AxelarMemoInstruction::SendToGateway {
            memo: "\u{1f42a}\u{1f42a}\u{1f42a}\u{1f42a}".to_string(),
            destination_chain: "ethereum".to_string(),
            destination_address: "0x0".to_string(),
        })
        .unwrap(),
    );

    let mut memo_instruction_bytes = Vec::new();
    memo_instruction
        .serialize(&mut memo_instruction_bytes)
        .unwrap();

    IxBuilder::new().with_proposal_data(
        axelar_solana_memo_program_old::ID,
        native_value,
        default_proposal_eta(),
        native_target_value_account,
        solana_accounts,
        memo_instruction_bytes,
    )
}

pub(crate) fn gmp_memo_metadata() -> GmpMetadata {
    GmpMetadata {
        cross_chain_id: CrossChainId::new(
            SOURCE_CHAIN_NAME.to_string(),
            uuid::Uuid::new_v4().to_string(),
        ),
        source_address: SOURCE_CHAIN_ADDRESS.to_string(),
        destination_address: axelar_solana_governance::ID.to_string(),
        destination_chain: "solana".to_string(),
        domain_separator: [0_u8; 32],
    }
}

pub(crate) fn events(
    tx: &solana_program_test::BanksTransactionResultWithMetadata,
) -> Vec<EventContainer> {
    tx.metadata
        .as_ref()
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GovernanceEvent::parse_log)
        .collect::<Vec<_>>()
}

pub(crate) fn assert_msg_present_in_logs(res: BanksTransactionResultWithMetadata, msg: &str) {
    assert!(
        res.metadata
            .unwrap()
            .log_messages
            .into_iter()
            .any(|x| x.contains(msg)),
        "Expected error message not found!"
    );
}

pub(crate) async fn approve_ix_at_gateway(
    sol_integration: &mut SolanaAxelarIntegrationMetadata,
    ix: &mut Instruction,
    meta: GmpMetadata,
) {
    let ((gateway_approved_message_pda, _, _), cmd_id) =
        approve_ix_data_at_gateway(meta, ix, sol_integration).await;
    let signing_pda = DestinationProgramId(axelar_solana_governance::id());
    builder::prepend_gateway_accounts_to_ix(
        ix,
        sol_integration.gateway_root_pda,
        gateway_approved_message_pda[0],
        signing_pda.signing_pda(&cmd_id).0,
    );
}

async fn approve_ix_data_at_gateway(
    meta: GmpMetadata,
    ix: &Instruction,
    sol_integration: &mut SolanaAxelarIntegrationMetadata,
) -> ((Vec<Pubkey>, Vec<u8>, Pubkey), [u8; 32]) {
    let message = message_with_meta_and_payload(meta, &ix.data);
    let cmd_id = message.cc_id().command_id(hasher_impl());
    let res = sol_integration
        .fixture
        .fully_approve_messages(
            &sol_integration.gateway_root_pda,
            vec![message],
            &sol_integration.signers,
            &sol_integration.domain_separator,
        )
        .await;
    (res, cmd_id)
}

fn message_with_meta_and_payload(meta: GmpMetadata, payload: &[u8]) -> Message {
    let payload_hash = solana_program::keccak::hash(payload).to_bytes();
    Message::new(
        meta.cross_chain_id,
        meta.source_address,
        meta.destination_chain,
        meta.destination_address,
        payload_hash,
        meta.domain_separator,
    )
}
