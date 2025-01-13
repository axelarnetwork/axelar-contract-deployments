use axelar_solana_gateway::error::GatewayError;
use axelar_solana_gateway::instructions::GatewayInstruction;
use axelar_solana_gateway::processor::{GatewayEvent, OperatorshipTransferredEvent};
use axelar_solana_gateway::state::GatewayConfig;
use axelar_solana_gateway_test_fixtures::base::TestFixture;
use axelar_solana_gateway_test_fixtures::gateway::{get_gateway_events, ProgramInvocationState};
use axelar_solana_gateway_test_fixtures::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};
use num_traits::ToPrimitive as _;
use program_utils::BytemuckedPda;
use solana_program_test::tokio::fs;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::account::ReadableAccount;
use solana_sdk::bpf_loader_upgradeable;
use solana_sdk::instruction::{AccountMeta, Instruction, InstructionError};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::TransactionError;

#[tokio::test]
async fn successfully_transfer_operatorship_when_signer_is_operator() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        gateway_root_pda,
        operator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .build()
        .setup()
        .await;
    let new_operator = Keypair::new();
    let original_config_acc = fixture
        .get_account(&gateway_root_pda, &axelar_solana_gateway::ID)
        .await;
    let original_config = GatewayConfig::read(original_config_acc.data()).unwrap();

    // Action
    let ix = axelar_solana_gateway::instructions::transfer_operatorship(
        gateway_root_pda,
        operator.pubkey(),
        new_operator.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers(&[ix], &[&operator, &fixture.payer.insecure_clone()])
        .await
        .unwrap();

    // Assert
    assert!(tx.result.is_ok());
    // - expected events
    let emitted_events = get_gateway_events(&tx).pop().unwrap();
    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };
    let [(_, GatewayEvent::OperatorshipTransferred(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };
    let expected_event = OperatorshipTransferredEvent {
        new_operator: new_operator.pubkey(),
    };
    assert_eq!(emitted_event, &expected_event);
    // - command PDAs get updated
    let altered_config_acc = fixture
        .get_account(&gateway_root_pda, &axelar_solana_gateway::ID)
        .await;

    let altered_config = GatewayConfig::read(altered_config_acc.data()).unwrap();

    let mut expected_config = *original_config;
    expected_config.operator = altered_config.operator;

    assert_eq!(*altered_config, expected_config);
}

// succeed if signer is gateway program owner
#[tokio::test]
async fn successfully_transfer_operatorship_when_signer_is_upgrade_authority() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        gateway_root_pda,
        upgrade_authority,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .build()
        .setup()
        .await;

    let original_config_acc = fixture
        .get_account(&gateway_root_pda, &axelar_solana_gateway::ID)
        .await;
    let original_config = GatewayConfig::read(original_config_acc.data()).unwrap();

    // Action - upgrade authority signs message to change operator
    let new_operator = Keypair::new();
    let ix = axelar_solana_gateway::instructions::transfer_operatorship(
        gateway_root_pda,
        upgrade_authority.pubkey(),
        new_operator.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                &upgrade_authority.insecure_clone(),
                &fixture.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    // Assert
    assert!(tx.result.is_ok());
    // - expected events
    let emitted_events = get_gateway_events(&tx).pop().unwrap();
    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };
    let [(_, GatewayEvent::OperatorshipTransferred(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };
    let expected_event = OperatorshipTransferredEvent {
        new_operator: new_operator.pubkey(),
    };
    assert_eq!(emitted_event, &expected_event);
    // - command PDAs get updated
    let altered_config_acc = fixture
        .get_account(&gateway_root_pda, &axelar_solana_gateway::ID)
        .await;

    let altered_config = GatewayConfig::read(altered_config_acc.data()).unwrap();

    let mut expected_config = *original_config;
    expected_config.operator = altered_config.operator;
    assert_eq!(*altered_config, expected_config);
}

// fail if gateway not initialized
#[tokio::test]
async fn fail_if_gateway_not_initialised() {
    // Setup
    // Create a new ProgramTest instance
    let mut fixture = TestFixture::new(ProgramTest::default()).await;
    // Generate a new keypair for the upgrade authority
    let upgrade_authority = Keypair::new();
    let gateway_program_bytecode = fs::read("../../target/deploy/axelar_solana_gateway.so")
        .await
        .unwrap();
    fixture
        .register_upgradeable_program(
            &gateway_program_bytecode,
            &upgrade_authority.pubkey(),
            &axelar_solana_gateway::id(),
        )
        .await;
    let (gateway_root_pda, ..) = axelar_solana_gateway::get_gateway_root_config_pda();
    // Action - upgrade authority signs message to change operator
    let new_operator = Keypair::new();
    let ix = axelar_solana_gateway::instructions::transfer_operatorship(
        gateway_root_pda,
        upgrade_authority.pubkey(),
        new_operator.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[&upgrade_authority, &fixture.payer.insecure_clone()],
        )
        .await
        .unwrap_err();

    // Assert
    assert!(tx.result.is_err());
    // - expected events
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        // todo: improve error message
        .any(|msg| { msg.contains("insufficient funds for instruction") }));
}

#[tokio::test]
async fn fail_if_operator_or_owner_does_not_match() {
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        gateway_root_pda,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .build()
        .setup()
        .await;

    // Action - random wallet signs message to change operator
    let stranger_danger = Keypair::new();
    let new_operator = Keypair::new();
    let ix = axelar_solana_gateway::instructions::transfer_operatorship(
        gateway_root_pda,
        stranger_danger.pubkey(), // this keypair is not a valid signer
        new_operator.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers(&[ix], &[&stranger_danger, &fixture.payer.insecure_clone()])
        .await
        .unwrap_err();

    // Assert
    assert!(tx.result.is_err());
    // - expected events
    let Err(TransactionError::InstructionError(_index, InstructionError::Custom(error_code))) =
        tx.result
    else {
        panic!("unexpected error")
    };

    assert_eq!(
        error_code,
        GatewayError::InvalidOperatorOrAuthorityAccount
            .to_u32()
            .unwrap()
    );
}

// fail if program id does not match gateway program id when deriving the
// programdata account
#[tokio::test]
async fn fail_if_invalid_program_id() {
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        gateway_root_pda,
        upgrade_authority,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .build()
        .setup()
        .await;

    // Action - we provide an invalid program id that is used for deriving the
    // program upgrade authority
    let new_operator = Pubkey::new_unique();
    let invalid_program_id = Pubkey::new_unique();
    let (programdata_pubkey, _) = Pubkey::try_find_program_address(
        &[invalid_program_id.as_ref()], // this is the baddie!
        &bpf_loader_upgradeable::id(),
    )
    .unwrap();
    let accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new_readonly(upgrade_authority.pubkey(), true),
        AccountMeta::new_readonly(programdata_pubkey, false),
        AccountMeta::new_readonly(new_operator, false),
    ];

    let data = borsh::to_vec(&GatewayInstruction::TransferOperatorship).unwrap();

    let ix = Instruction {
        program_id: axelar_solana_gateway::id(),
        accounts,
        data,
    };
    let tx = fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[&upgrade_authority, &fixture.payer.insecure_clone()],
        )
        .await
        .unwrap_err();

    // Assert
    assert!(tx.result.is_err());
    // - expected events
    let Err(TransactionError::InstructionError(_index, InstructionError::Custom(error_code))) =
        tx.result
    else {
        panic!("unexpected error")
    };

    assert_eq!(
        error_code,
        GatewayError::InvalidProgramDataDerivation.to_u32().unwrap()
    );
}

// the stranger does not actually sign the tx
#[tokio::test]
async fn fail_if_stranger_dose_not_sing_anything() {
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        gateway_root_pda,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .build()
        .setup()
        .await;

    // Action - the stranger does not actually sign the tx, he just hopes there's
    // not a check
    let stranger_danger = Keypair::new();
    let new_operator = Pubkey::new_unique();
    let (programdata_pubkey, _) = Pubkey::try_find_program_address(
        &[axelar_solana_gateway::id().as_ref()],
        &bpf_loader_upgradeable::id(),
    )
    .unwrap();
    let accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new_readonly(stranger_danger.pubkey(), false), /* the `false` flag is the
                                                                     * "off" thing here which
                                                                     * allows to not sign the tx
                                                                     * on he "client" side */
        AccountMeta::new_readonly(programdata_pubkey, false),
        AccountMeta::new_readonly(new_operator, false),
    ];

    let data = borsh::to_vec(&GatewayInstruction::TransferOperatorship).unwrap();

    let ix = Instruction {
        program_id: axelar_solana_gateway::id(),
        accounts,
        data,
    };
    let tx = fixture
        .send_tx_with_custom_signers(&[ix], &[&&fixture.payer.insecure_clone()])
        .await
        .unwrap_err();

    // Assert
    assert!(tx.result.is_err());
    // - expected events
    let Err(TransactionError::InstructionError(_index, InstructionError::Custom(error_code))) =
        tx.result
    else {
        panic!("unexpected error")
    };

    assert_eq!(
        error_code,
        GatewayError::OperatorOrUpgradeAuthorityMustBeSigner
            .to_u32()
            .unwrap()
    );
}
