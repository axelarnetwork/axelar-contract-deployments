use axelar_rkyv_encoding::hasher::solana::SolanaKeccak256Hasher;
use axelar_rkyv_encoding::test_fixtures::{
    random_bytes, random_message, random_valid_execute_data_and_verifier_set_for_payload,
};
use axelar_rkyv_encoding::types::Payload;
use gmp_gateway::commands::{CommandKind, OwnedCommand};
use gmp_gateway::state::execute_data_buffer::BufferLayout;
use gmp_gateway::state::{ApprovedMessageStatus, GatewayApprovedCommand};
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::test_setup::{
    make_signers, SigningVerifierSet, SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
    TestFixture,
};

use crate::{gateway_approved_command_ixs, make_payload_and_commands, program_test};

#[tokio::test]
async fn successfully_initialize_validate_message_command() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![10, 4])
        .build()
        .setup()
        .await;

    let (payload, commands) = make_payload_and_commands(3);
    fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    // Action
    let ixs = gateway_approved_command_ixs(&commands, gateway_root_pda, &fixture);
    let gateway_approved_command_pdas = ixs.iter().map(|(pda, _)| *pda).collect::<Vec<_>>();
    let ixs = ixs.into_iter().map(|(_, ix)| ix).collect::<Vec<_>>();
    fixture.send_tx(&ixs).await;

    // Assert
    for pda in gateway_approved_command_pdas {
        let account = fixture
            .banks_client
            .get_account(pda)
            .await
            .expect("call failed")
            .expect("account not found");
        let gateway_approved_command = account
            .check_initialized_pda::<GatewayApprovedCommand>(&gmp_gateway::id())
            .unwrap();
        assert!(!gateway_approved_command.is_command_executed());
        assert!(!gateway_approved_command.is_validate_message_executed());
        assert!(matches!(
            gateway_approved_command.status(),
            ApprovedMessageStatus::Pending
        ));
    }
}

#[tokio::test]
async fn fail_when_gateway_root_pda_not_initialized() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let gateway_root_pda = Pubkey::new_unique();

    let (_, commands) = make_payload_and_commands(1);

    let ixs = gateway_approved_command_ixs(&commands, gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect::<Vec<_>>();
    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&ixs).await;

    // Assert
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        // This means that the account was not initialized - has 0 lamports
        .any(|x| x.contains("insufficient funds for instruction")),);
}

#[tokio::test]
async fn successfully_initialize_command_which_belongs_to_a_different_execute_data_set() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![10, 4])
        .build()
        .setup()
        .await;

    let (payload_1, _) = make_payload_and_commands(1);
    let (_execute_data_pubkey_1, _execute_data_1) = fixture
        .init_execute_data(&gateway_root_pda, payload_1, &signers, &domain_separator)
        .await;
    let (_payload_2, commands_2) = make_payload_and_commands(1);

    // Action
    let (pdas, ixs): (Vec<_>, Vec<_>) =
        gateway_approved_command_ixs(&commands_2, gateway_root_pda, &fixture)
            .into_iter()
            .unzip();
    fixture.send_tx(&ixs).await;

    // Assert
    for pda in pdas {
        let account = fixture
            .banks_client
            .get_account(pda)
            .await
            .expect("call failed")
            .expect("account not found");
        let gateway_approved_command = account
            .check_initialized_pda::<GatewayApprovedCommand>(&gmp_gateway::id())
            .unwrap();
        assert!(!gateway_approved_command.is_command_executed());
        assert!(!gateway_approved_command.is_validate_message_executed());
        assert!(matches!(
            gateway_approved_command.status(),
            ApprovedMessageStatus::Pending
        ));
    }
}

#[tokio::test]
async fn fail_when_validate_message_already_initialized() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![10, 4])
        .build()
        .setup()
        .await;

    let (payload, commands) = make_payload_and_commands(1);
    fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    let ixs = gateway_approved_command_ixs(&commands, gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect::<Vec<_>>();
    fixture.send_tx(&ixs).await;

    // Action -- will fail when trying to initialize the same command
    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&ixs).await;

    // Assert
    //
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        // this means that the account was already initialized
        // TODO: improve error message
        .any(|x| x.contains("invalid account data for instruction")),);
}

#[tokio::test]
async fn fail_when_rotate_signers_is_already_initialized() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![10, 4])
        .build()
        .setup()
        .await;

    let new_signer_set = make_signers(&[44], 44);
    let payload = Payload::VerifierSet(new_signer_set.verifier_set().clone());
    let command = OwnedCommand::RotateSigners(new_signer_set.verifier_set());
    fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    let ixs: Vec<_> = gateway_approved_command_ixs(&[command], gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect();
    fixture.send_tx(&ixs).await;

    // Action -- will fail when trying to initialize the same command
    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&ixs).await;

    // Assert
    //
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        // this means that the account was already initialized
        // TODO: improve error message
        .any(|x| x.contains("invalid account data for instruction")),);
}

#[tokio::test]
async fn succeed_when_same_signers_with_different_nonce_get_initialized() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![10, 4])
        .build()
        .setup()
        .await;

    // Signer set B is equal to A but with a different nonce.
    let signer_set_a = make_signers(&[10u128, 4], 10);
    let signer_set_b = SigningVerifierSet {
        nonce: 55,
        ..signer_set_a.clone()
    };

    // Payloads
    let payload_a = Payload::VerifierSet(signer_set_a.clone().verifier_set());
    let payload_b = Payload::VerifierSet(signer_set_b.clone().verifier_set());

    // Commands
    let command_a = OwnedCommand::RotateSigners(signer_set_a.verifier_set());
    let command_b = OwnedCommand::RotateSigners(signer_set_b.verifier_set());

    fixture
        .init_execute_data(&gateway_root_pda, payload_a, &signers, &domain_separator)
        .await;
    fixture
        .init_execute_data(&gateway_root_pda, payload_b, &signers, &domain_separator)
        .await;
    let ixs_a = gateway_approved_command_ixs(&[command_a], gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect::<Vec<_>>();
    fixture.send_tx(&ixs_a).await;

    // Action
    let ixs_b = gateway_approved_command_ixs(&[command_b], gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect::<Vec<_>>();
    let BanksTransactionResultWithMetadata {
        metadata: _,
        result,
    } = fixture.send_tx_with_metadata(&ixs_b).await;

    // Assert
    assert!(result.is_ok(), "Transaction should not have failed");
}

struct BufferedWriteTestCase {
    fixture: TestFixture,
    gateway_root_pda: Pubkey,
    num_chunks: usize,
    execute_data_bytes: Vec<u8>,
    payload_hash: [u8; 32],
    user_seed: [u8; 32],
    buffer_account: Pubkey,
    bump_seed: u8,
}

impl BufferedWriteTestCase {
    async fn new(num_messages: usize, signer_weights: &[u128], num_chunks: usize) -> Self {
        let SolanaAxelarIntegrationMetadata {
            fixture,
            gateway_root_pda,
            domain_separator,
            ..
        } = SolanaAxelarIntegration::builder()
            .initial_signer_weights(signer_weights.to_vec())
            .build()
            .setup()
            .await;

        // Generate the test `execute_data` and its payload hash
        let (execute_data_bytes, payload_hash) = {
            let messages = (0..num_messages).map(|_| random_message()).collect();
            let payload = Payload::new_messages(messages);
            let (execute_data, verifier_set) =
                random_valid_execute_data_and_verifier_set_for_payload(
                    &domain_separator,
                    payload.clone(),
                );
            let execute_data_bytes = execute_data.to_bytes::<1024>().unwrap();
            let payload_hash = axelar_rkyv_encoding::hash_payload(
                &domain_separator,
                &verifier_set,
                &payload,
                SolanaKeccak256Hasher::default(),
            );
            dbg!(execute_data_bytes.len());
            (execute_data_bytes, payload_hash)
        };

        let user_seed = random_bytes::<32>();
        let (buffer_account, bump_seed) =
            gmp_gateway::get_execute_data_pda(&gateway_root_pda, &user_seed);
        Self {
            fixture,
            gateway_root_pda,
            num_chunks,
            execute_data_bytes,
            payload_hash,
            user_seed,
            buffer_account,
            bump_seed,
        }
    }

    async fn run(&mut self) {
        // Split the `execute_data` in three chunks
        let write_ixs = split(&self.execute_data_bytes, self.num_chunks).map(|chunk| {
            gmp_gateway::instructions::write_execute_data_buffer(
                self.gateway_root_pda,
                &self.user_seed,
                self.bump_seed,
                chunk.data,
                chunk.offset,
            )
            .unwrap()
        });

        // Prepare instructions
        let mut ixs = vec![];
        ixs.push(
            gmp_gateway::instructions::initialize_execute_data_buffer(
                self.gateway_root_pda,
                self.fixture.payer.pubkey(),
                self.execute_data_bytes.len() as u64,
                self.user_seed,
                CommandKind::ApproveMessage,
            )
            .unwrap(),
        );
        ixs.extend(write_ixs);
        ixs.push(
            gmp_gateway::instructions::finalize_execute_data_buffer(
                self.gateway_root_pda,
                &self.user_seed,
                self.bump_seed,
            )
            .unwrap(),
        );

        // Confidence check: We really used `num_chunks` + 2 instructions
        assert_eq!(ixs.len(), 2 + self.num_chunks);

        // Send one transaction per instruction
        for instruction in ixs {
            self.send_individual_transaction(instruction).await;
        }

        // Check: Final account data matches what we wrote
        let mut buffer_account_data = self
            .fixture
            .banks_client
            .get_account(self.buffer_account)
            .await
            .expect("call failed")
            .expect("account not found")
            .data;

        let buffer = BufferLayout::parse(&mut buffer_account_data)
            .expect("failed to parse buffer account data");
        assert!(buffer.metadata().is_finalized());
        assert_eq!(buffer.raw_execute_data, self.execute_data_bytes);
        assert_eq!(*buffer.payload_hash, self.payload_hash);
    }

    async fn send_individual_transaction(&mut self, instruction: Instruction) {
        let BanksTransactionResultWithMetadata { result, .. } =
            self.fixture.send_tx_with_metadata(&[instruction]).await;

        // Check: Transaction success
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_buffered_execute_data_lifecycle() {
    // TODO: Experiment with other sizes.
    let signer_weights = &[10u128, 4];

    // This is our current limit.
    // Split count doesn't seem to interfere with it.
    //  Commenting it out because it can be flaky sometimes.
    /*
        BufferedWriteTestCase::new(40, signer_weights, 4)
            .await
            .run()
            .await;
    // */

    let magic_sequence = &[1, 2, 3, 5, 8, 13, 21, 34];
    for magic_number in magic_sequence {
        let num_messages = *magic_number;
        let num_chunks = magic_number * 3;
        BufferedWriteTestCase::new(num_messages, signer_weights, num_chunks)
            .await
            .run()
            .await
    }
}

/// Helper function to split a slice in `n` parts as evenly as possible
fn split<T>(slice: &[T], n: usize) -> impl Iterator<Item = ChunkWithOffset<'_, T>> {
    Split {
        slice,
        length: slice.len() / n,
        remainder: slice.len() % n,
        offset: 0,
    }
}

struct Split<'a, T> {
    slice: &'a [T],
    length: usize,
    remainder: usize,
    offset: usize,
}

struct ChunkWithOffset<'a, T> {
    data: &'a [T],
    offset: usize,
}

impl<'a, T> Iterator for Split<'a, T> {
    type Item = ChunkWithOffset<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None;
        }

        let mut length = self.length;
        if self.remainder > 0 {
            length += 1;
            self.remainder -= 1;
        }
        let (chunk, rest) = self.slice.split_at(length);

        let chunk_with_offset = ChunkWithOffset {
            data: chunk,
            offset: self.offset,
        };

        self.slice = rest;
        self.offset += length;

        Some(chunk_with_offset)
    }
}
