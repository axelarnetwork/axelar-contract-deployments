use axelar_rkyv_encoding::hasher::merkle_tree::{MerkleTree, SolanaSyscallHasher};
use axelar_rkyv_encoding::hasher::solana::SolanaKeccak256Hasher;
use axelar_rkyv_encoding::test_fixtures::{
    random_bytes, random_message, random_valid_execute_data_and_verifier_set_for_payload,
};
use axelar_rkyv_encoding::types::{
    ArchivedExecuteData, ArchivedPublicKey, ArchivedWeightedSigner, Payload,
};
use gmp_gateway::commands::{CommandKind, OwnedCommand};
use gmp_gateway::state::execute_data_buffer::BufferLayout;
use gmp_gateway::state::signature_verification::{
    batch_context_from_proof, BatchContext, SignatureNode, SignatureVerification,
};
use gmp_gateway::state::{ApprovedMessageStatus, GatewayApprovedCommand};
use itertools::Itertools;
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

    let new_signer_set = make_signers(&[44], 44, domain_separator);
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
    let signer_set_a = make_signers(&[10u128, 4], 10, domain_separator);
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
    domain_separator: [u8; 32],
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
                    domain_separator,
                    payload.clone(),
                );
            let execute_data_bytes = execute_data.to_bytes::<1024>().unwrap();
            let payload_hash = axelar_rkyv_encoding::hash_payload(
                &domain_separator,
                &verifier_set,
                &payload,
                SolanaKeccak256Hasher::default(),
            );
            (execute_data_bytes, payload_hash)
        };

        let user_seed = random_bytes::<32>();
        let (buffer_account, bump_seed) =
            gmp_gateway::get_execute_data_pda(&gateway_root_pda, &user_seed);
        Self {
            fixture,
            gateway_root_pda,
            domain_separator,
            num_chunks,
            execute_data_bytes,
            payload_hash,
            user_seed,
            buffer_account,
            bump_seed,
        }
    }

    /// Runs the full test case.
    async fn run_test(&mut self) {
        self.run_prepare_execute_data_for_signature_verification()
            .await;
        self.run_signature_verification().await;
        self.run_finalize().await;
    }

    /// Verifies all signatures for a given command batch.
    /// Instructions sent by this function:
    /// - 1: InitializeSignatureVerification
    /// - N: VerifySignature, where N is the number of signers in the batch.
    async fn run_signature_verification(&mut self) {
        // Setup
        let archived_execute_data = self.archived_execute_data();
        let batch_context = self.batch_context(archived_execute_data);
        let signatures_merkle_tree = self.build_signatures_merkle_tree();
        let signature_merkle_root = signatures_merkle_tree
            .root()
            .expect("test merkle tree should have at least one node");

        // `VerifySignature` instruction iterator
        let validate_signature_ixs = self
            .signature_leaf_nodes(archived_execute_data, &batch_context)
            .enumerate()
            .map(|(position, signature_leaf_node)| {
                let signature_merkle_proof = signatures_merkle_tree.proof(&[position]);
                let signature_leaf_node_hash = signature_leaf_node.hash();

                // Confidence check: Produced signature node and proof is valid
                assert!(
                    signature_merkle_proof.verify(
                        signature_merkle_root,
                        &[position],
                        &[signature_leaf_node_hash],
                        batch_context.signer_count as usize,
                    ),
                    "prepared signature node failed preflight inclusion check"
                );

                let (signature_bytes, public_key_bytes, signer_weight, signer_index) =
                    signature_leaf_node.into_parts();

                gmp_gateway::instructions::verify_signature(
                    self.gateway_root_pda,
                    &self.user_seed,
                    self.bump_seed,
                    signature_bytes,
                    public_key_bytes,
                    signer_weight,
                    signer_index,
                    signature_merkle_proof.to_bytes(),
                )
                .unwrap()
            });

        let mut ixs = Vec::new();
        ixs.push(
            gmp_gateway::instructions::initialize_signature_verification(
                self.gateway_root_pda,
                &self.user_seed,
                self.bump_seed,
                signature_merkle_root,
            )
            .unwrap(),
        );
        ixs.extend(validate_signature_ixs);

        for instruction in ixs {
            self.send_individual_transaction(instruction).await;
        }

        // Check if buffer account data changed as expected
        let mut buffer_account_data = self.fetch_buffer_account_data().await;
        let buffer = BufferLayout::parse(&mut buffer_account_data)
            .expect("failed to parse buffer account data");

        // Check signatures merkle tree
        let sig_verification = SignatureVerification::deserialize(buffer.signature_verification);
        assert_eq!(
            sig_verification.root(),
            self.calculate_signatures_merkle_root(),
            "on-chain signature merkle root should be the same as the one we calculated off-chain"
        );

        assert!(
            sig_verification.is_valid(),
            "not all signatures have been verified, but they should be by now"
        );
    }

    /// Finalizes the `execute_data_buffer`.
    ///
    /// Instructions sent by this function:
    /// - 1: FinalizeExecuteDataBuffer
    async fn run_finalize(&mut self) {
        let finalize_ix = gmp_gateway::instructions::finalize_execute_data_buffer(
            self.gateway_root_pda,
            &self.user_seed,
            self.bump_seed,
        )
        .unwrap();

        self.send_individual_transaction(finalize_ix).await;

        // Check if buffer account data changed as expected
        let mut buffer_account_data = self.fetch_buffer_account_data().await;
        let buffer = BufferLayout::parse(&mut buffer_account_data)
            .expect("failed to parse buffer account data");
        assert!(
            buffer.metadata().is_finalized(),
            "buffer should be finalized by now"
        );
    }

    /// Sends the full `execute_data` bytes in chunks to the Gateway.
    ///
    /// Instructions sent by this function:
    /// - 1: InitializeExecuteDataBuffer
    /// - N: WriteExecuteDataBuffer, where N = `self.num_chunks`
    /// - 1: CommitPayloadHash
    async fn run_prepare_execute_data_for_signature_verification(&mut self) {
        // Split the `execute_data` into chunks
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
        let mut ixs = vec![gmp_gateway::instructions::initialize_execute_data_buffer(
            self.gateway_root_pda,
            self.fixture.payer.pubkey(),
            self.execute_data_bytes.len() as u64,
            self.user_seed,
            CommandKind::ApproveMessage,
        )
        .unwrap()];
        ixs.extend(write_ixs);
        ixs.push(
            gmp_gateway::instructions::commit_payload_hash(
                self.gateway_root_pda,
                &self.user_seed,
                self.bump_seed,
            )
            .unwrap(),
        );

        // Confidence check: We really used `num_chunks` + 2 instructions
        assert_eq!(
            ixs.len(),
            2 + self.num_chunks,
            "an unexpected number of instructions was used"
        );

        // Send one transaction per instruction
        for instruction in ixs {
            self.send_individual_transaction(instruction).await;
        }
        // Check if buffer account data changed as expected
        let mut buffer_account_data = self.fetch_buffer_account_data().await;
        let buffer = BufferLayout::parse(&mut buffer_account_data)
            .expect("failed to parse buffer account data");
        assert!(buffer.metadata().has_payload_hash());
        assert!(!buffer.metadata().is_finalized());
        assert_eq!(buffer.raw_execute_data, self.execute_data_bytes);
        assert_eq!(*buffer.payload_hash, self.payload_hash);
    }

    async fn fetch_buffer_account_data(&mut self) -> Vec<u8> {
        self.fixture
            .banks_client
            .get_account(self.buffer_account)
            .await
            .expect("call failed")
            .expect("account not found")
            .data
    }

    async fn send_individual_transaction(&mut self, instruction: Instruction) {
        let BanksTransactionResultWithMetadata { result, .. } =
            self.fixture.send_tx_with_metadata(&[instruction]).await;

        // Check: Transaction success
        assert!(result.is_ok());
    }

    fn archived_execute_data(&self) -> &ArchivedExecuteData {
        ArchivedExecuteData::from_bytes(&self.execute_data_bytes)
            .expect("test should use valid execute_data bytes")
    }

    fn batch_context(&self, archived_execute_data: &ArchivedExecuteData) -> BatchContext {
        batch_context_from_proof(
            self.gateway_root_pda,
            self.domain_separator,
            &archived_execute_data.proof,
            self.payload_hash,
        )
        .expect("test should parse batch context from proof")
    }

    fn signers_with_signatures<'a>(
        &'a self,
        archived_execute_data: &'a ArchivedExecuteData,
    ) -> impl Iterator<Item = (&'a ArchivedPublicKey, &'a ArchivedWeightedSigner)> {
        archived_execute_data.proof.signers_with_signatures.iter()
    }

    fn signature_leaf_nodes<'a>(
        &'a self,
        archived_execute_data: &'a ArchivedExecuteData,
        batch_context: &'a BatchContext,
    ) -> impl Iterator<Item = SignatureNode<'a, Vec<u8>, Vec<u8>>> + 'a {
        self.signers_with_signatures(archived_execute_data)
            .enumerate()
            .map(|(signer_index, (signer_pubkey, weighted_signer))| {
                let public_key_bytes = signer_pubkey.to_bytes();
                let signature_bytes: Vec<u8> = weighted_signer
                    .signature
                    .as_ref()
                    .map(|signature| signature.as_ref().into())
                    .unwrap_or_default();
                let signer_weight = (&weighted_signer.weight).into();

                SignatureNode::new(
                    signature_bytes,
                    public_key_bytes,
                    signer_weight,
                    signer_index.try_into().expect("usize to fit into an u8"),
                    batch_context,
                )
            })
    }

    fn build_signatures_merkle_tree(&self) -> MerkleTree<SolanaSyscallHasher> {
        let archived_execute_data = self.archived_execute_data();
        let batch_context = &self.batch_context(archived_execute_data);
        let leaves = self
            .signature_leaf_nodes(archived_execute_data, batch_context)
            .collect_vec();
        SignatureVerification::build_merkle_tree::<_, _>(leaves.as_slice())
    }

    fn calculate_signatures_merkle_root(&self) -> [u8; 32] {
        self.build_signatures_merkle_tree()
            .root()
            .expect("test merkle tree should have at least one node")
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
            .run_test()
            .await;
    // */

    let magic_sequence = &[1, 2, 3, 5, 8, 13, 21, 34];
    for magic_number in magic_sequence {
        let num_messages = *magic_number;
        let num_chunks = magic_number * 3;
        BufferedWriteTestCase::new(num_messages, signer_weights, num_chunks)
            .await
            .run_test()
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
