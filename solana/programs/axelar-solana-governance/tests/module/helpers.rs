use std::time::SystemTime;

use axelar_solana_encoding::types::messages::{CrossChainId, Message};
use axelar_solana_gateway::state::incoming_message::command_id;
use axelar_solana_gateway_test_fixtures::base::TestFixture;
use axelar_solana_gateway_test_fixtures::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};
use axelar_solana_governance::events::{EventContainer, GovernanceEvent};
use axelar_solana_governance::instructions::builder::{
    prepend_gateway_accounts_to_ix, GmpCallData, IxBuilder,
};
use axelar_solana_governance::state::GovernanceConfig;
use axelar_solana_memo_program::instruction::AxelarMemoInstruction;
use borsh::to_vec;
use solana_program_test::{processor, BanksTransactionResultWithMetadata, ProgramTest};
use solana_sdk::bpf_loader_upgradeable;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};

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

    let upgrade_authority = Keypair::new();

    // Setup gateway
    let mut sol_integration = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![555, 222])
        .programs_to_deploy(vec![(
            "axelar_solana_memo_program.so".into(),
            axelar_solana_memo_program::id(),
        )])
        .build()
        .setup_with_fixture_and_authority(fixture, upgrade_authority.insecure_clone())
        .await;

    // Immediately set the upgrade authority to the governance program. (needed for tests)
    let ix = bpf_loader_upgradeable::set_upgrade_authority(
        &axelar_solana_gateway::ID,
        &upgrade_authority.pubkey(),
        Some(&gov_config_pda),
    );

    let res = sol_integration
        .fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                upgrade_authority.insecure_clone(),
                sol_integration.payer.insecure_clone(),
            ],
        )
        .await;
    assert!(res.is_ok());

    let memo_counter_pda =
        axelar_solana_memo_program::get_counter_pda(&sol_integration.gateway_root_pda);
    let ix = axelar_solana_memo_program::instruction::initialize(
        &sol_integration.fixture.payer.pubkey(),
        &sol_integration.gateway_root_pda,
        &memo_counter_pda,
    )
    .unwrap();

    let res = sol_integration.fixture.send_tx(&[ix]).await;
    assert!(res.is_ok());

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
    fixture.send_tx(&[ix]).await.unwrap();
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

pub(crate) fn gmp_sample_metadata() -> Message {
    Message {
        cc_id: CrossChainId {
            chain: SOURCE_CHAIN_NAME.to_string(),
            id: uuid::Uuid::new_v4().to_string(),
        },
        source_address: SOURCE_CHAIN_ADDRESS.to_string(),
        destination_address: axelar_solana_governance::ID.to_string(),
        destination_chain: "solana".to_string(),
        payload_hash: [0_u8; 32],
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
    let memo_instruction = to_vec(&AxelarMemoInstruction::SendToGateway {
        memo: "\u{1f42a}\u{1f42a}\u{1f42a}\u{1f42a}".to_string(),
        destination_chain: "ethereum".to_string(),
        destination_address: "0x0".to_string(),
    })
    .unwrap();

    IxBuilder::new().with_proposal_data(
        axelar_solana_memo_program::ID,
        native_value,
        default_proposal_eta(),
        native_target_value_account,
        solana_accounts,
        memo_instruction,
    )
}

pub(crate) fn gmp_memo_metadata() -> Message {
    Message {
        cc_id: CrossChainId {
            chain: SOURCE_CHAIN_NAME.to_string(),
            id: uuid::Uuid::new_v4().to_string(),
        },
        source_address: SOURCE_CHAIN_ADDRESS.to_string(),
        destination_address: axelar_solana_governance::ID.to_string(),
        destination_chain: "solana".to_string(),
        payload_hash: [0_u8; 32],
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
    gmp_build: &mut GmpCallData,
) {
    let _message_from_multisig_prover = sol_integration
        .sign_session_and_approve_messages(
            &sol_integration.signers.clone(),
            &[gmp_build.msg_meta.clone()],
        )
        .await
        .unwrap();

    let message_payload_pda = sol_integration
        .upload_message_payload(&gmp_build.msg_meta, &gmp_build.msg_payload)
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) = axelar_solana_gateway::get_incoming_message_pda(&command_id(
        &gmp_build.msg_meta.cc_id.chain,
        &gmp_build.msg_meta.cc_id.id,
    ));

    prepend_gateway_accounts_to_ix(
        &mut gmp_build.ix,
        incoming_message_pda,
        message_payload_pda,
        &gmp_build.msg_meta,
    );
}
