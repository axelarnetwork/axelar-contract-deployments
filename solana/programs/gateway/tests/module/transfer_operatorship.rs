use gmp_gateway::instructions::GatewayInstruction;
use gmp_gateway::state::GatewayConfig;
use solana_program_test::tokio::fs;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::bpf_loader_upgradeable::{self};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use test_fixtures::test_setup::TestFixture;

use crate::{setup_initialised_gateway, InitialisedGatewayMetadata};

#[tokio::test]
async fn successfully_transfer_operatorship_when_signer_is_operator() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        gateway_root_pda,
        operator,
        ..
    } = setup_initialised_gateway(&[11, 42, 33], None).await;
    let new_operator = Keypair::new();
    let original_config = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;

    // Action
    let ix = gmp_gateway::instructions::transfer_operatorship(
        gateway_root_pda,
        operator.pubkey(),
        new_operator.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&operator, &fixture.payer.insecure_clone()],
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    // - expected events
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Operatorship transferred") }));
    // - command PDAs get updated
    let altered_config = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    assert_eq!(
        altered_config,
        GatewayConfig {
            operator: altered_config.operator,
            ..original_config
        }
    );
}

// succeed if signer is gateway program owner
#[tokio::test]
async fn successfully_transfer_operatorship_when_signer_is_upgrade_authority() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        gateway_root_pda,
        upgrade_authority,
        ..
    } = setup_initialised_gateway(&[11, 42, 33], None).await;
    let original_config = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;

    // Action - upgrade authority signs message to change operator
    let new_operator = Keypair::new();
    let ix = gmp_gateway::instructions::transfer_operatorship(
        gateway_root_pda,
        upgrade_authority.pubkey(),
        new_operator.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&upgrade_authority, &fixture.payer.insecure_clone()],
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    // - expected events
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Operatorship transferred") }));
    // - command PDAs get updated
    let altered_config = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    assert_eq!(
        altered_config,
        GatewayConfig {
            operator: altered_config.operator,
            ..original_config
        }
    );
}

// fail if gateway not initialied
#[tokio::test]
async fn fail_if_gateway_not_initialised() {
    // Setup
    // Create a new ProgramTest instance
    let mut fixture = TestFixture::new(ProgramTest::default()).await;
    // Generate a new keypair for the upgrade authority
    let upgrade_authority = Keypair::new();
    let gateway_program_bytecode = fs::read("../../target/deploy/gmp_gateway.so")
        .await
        .unwrap();
    fixture
        .register_upgradeable_program(
            &gateway_program_bytecode,
            &upgrade_authority.pubkey(),
            &gmp_gateway::id(),
        )
        .await;
    let (gateway_root_pda, ..) = GatewayConfig::pda();
    // Action - upgrade authority signs message to change operator
    let new_operator = Keypair::new();
    let ix = gmp_gateway::instructions::transfer_operatorship(
        gateway_root_pda,
        upgrade_authority.pubkey(),
        new_operator.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&upgrade_authority, &fixture.payer.insecure_clone()],
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    // - expected events
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        // todo: improve errror message
        .any(|msg| { msg.contains("insufficient funds for instruction") }));
}

#[tokio::test]
async fn fail_if_operator_or_owner_is_not_signer() {
    let InitialisedGatewayMetadata {
        mut fixture,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 42, 33], None).await;
    let original_config = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;

    // Action - random wallet signs message to change operator
    let stranger_danger = Keypair::new();
    let new_operator = Keypair::new();
    let ix = gmp_gateway::instructions::transfer_operatorship(
        gateway_root_pda,
        stranger_danger.pubkey(), // this keypair is not a valid singer
        new_operator.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&stranger_danger, &fixture.payer.insecure_clone()],
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    // - expected events
    assert!(tx.metadata.unwrap().log_messages.into_iter().any(|msg| {
        msg.contains(
            "Operator or owner account is not the factual operator or the owner of the Gateway",
        )
    }));
}

// fail if program id does not match gateway program id when deriving the
// programdata account
#[tokio::test]
async fn fail_if_invalid_program_id() {
    let InitialisedGatewayMetadata {
        mut fixture,
        gateway_root_pda,
        upgrade_authority,
        ..
    } = setup_initialised_gateway(&[11, 42, 33], None).await;

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
        program_id: gmp_gateway::id(),
        accounts,
        data,
    };
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&upgrade_authority, &fixture.payer.insecure_clone()],
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    // - expected events
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("invalid programdata account provided",) }));
}

// the stranger does not actually sign the tx
#[tokio::test]
async fn fail_if_stranger_dose_not_sing_anything() {
    let InitialisedGatewayMetadata {
        mut fixture,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 42, 33], None).await;

    // Action - the stranger does not  actually sign the tx, he just hopes ther's
    // not a check
    let stranger_danger = Keypair::new();
    let new_operator = Pubkey::new_unique();
    let (programdata_pubkey, _) = Pubkey::try_find_program_address(
        &[gmp_gateway::id().as_ref()],
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
        program_id: gmp_gateway::id(),
        accounts,
        data,
    };
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(&[ix], &[&&fixture.payer.insecure_clone()])
        .await;

    // Assert
    assert!(tx.result.is_err());
    // - expected events
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Operator or owner account must be a signer",) }));
}
