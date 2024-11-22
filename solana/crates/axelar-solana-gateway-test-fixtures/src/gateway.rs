//! Module that contains gateway specific utilities

use core::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

use axelar_solana_encoding::hasher::NativeHasher;
use axelar_solana_encoding::types::execute_data::{
    ExecuteData, MerkleisedMessage, MerkleisedPayload,
};
use axelar_solana_encoding::types::messages::{CrossChainId, Message, Messages};
use axelar_solana_encoding::types::payload::Payload;
use axelar_solana_encoding::types::verifier_set::{verifier_set_hash, VerifierSet};
use axelar_solana_encoding::{borsh, hash_payload};
use axelar_solana_gateway::events::{EventContainer, GatewayEvent};
use axelar_solana_gateway::state::incoming_message::{command_id, IncomingMessageWrapper};
use axelar_solana_gateway::state::signature_verification_pda::SignatureVerificationSessionData;
use axelar_solana_gateway::state::verifier_set_tracker::VerifierSetTracker;
use axelar_solana_gateway::state::GatewayConfig;
use axelar_solana_gateway::{
    bytemuck, get_gateway_root_config_pda, get_incoming_message_pda, get_verifier_set_tracker_pda,
};
use rand::Rng as _;
use solana_program::pubkey::Pubkey;
use solana_program_test::{BanksTransactionResultWithMetadata, ProgramTest};
use solana_sdk::account::ReadableAccount as _;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer as _;

use crate::base::{workspace_root_dir, TestFixture};
use crate::test_signer::{create_signer_with_weight, SigningVerifierSet};

/// Contains metadata information about the initialised Gateway config
pub struct SolanaAxelarIntegrationMetadata {
    /// the underlying test fixture
    pub fixture: TestFixture,
    /// the initial verifier set
    pub signers: SigningVerifierSet,
    /// gateway root pda
    pub gateway_root_pda: Pubkey,
    /// the initial operator
    pub operator: Keypair,
    /// upgrade authority for the gateway program
    pub upgrade_authority: Keypair,
    /// domain separator that the gateway was instandiated with
    pub domain_separator: [u8; 32],
    /// the verifier retention
    pub previous_signers_retention: u64,
    /// minimum signer rotation delay between calls
    pub minimum_rotate_signers_delay_seconds: u64,
}

impl core::ops::Deref for SolanaAxelarIntegrationMetadata {
    type Target = TestFixture;

    fn deref(&self) -> &Self::Target {
        &self.fixture
    }
}

impl core::ops::DerefMut for SolanaAxelarIntegrationMetadata {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fixture
    }
}

impl SolanaAxelarIntegrationMetadata {
    /// Get the gateway init verifier set data.
    /// This is useful for building the instantiation message for the gateway
    #[must_use]
    pub fn init_gateway_config_verifier_set_data(&self) -> Vec<([u8; 32], Pubkey, u8)> {
        let init_signers_hash =
            verifier_set_hash::<NativeHasher>(&self.signers.verifier_set(), &self.domain_separator)
                .unwrap();
        let (initial_signers_pda, initial_signers_bump) = self.signers.verifier_set_tracker();
        vec![(init_signers_hash, initial_signers_pda, initial_signers_bump)]
    }

    /// Initialise the gateway root config
    pub async fn initialize_gateway_config_account(
        &mut self,
    ) -> Result<Pubkey, BanksTransactionResultWithMetadata> {
        let (gateway_config_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
        let initial_signer_sets = self.init_gateway_config_verifier_set_data();
        let ix = axelar_solana_gateway::instructions::initialize_config(
            self.fixture.payer.pubkey(),
            self.domain_separator,
            initial_signer_sets,
            self.minimum_rotate_signers_delay_seconds,
            self.operator.pubkey(),
            self.previous_signers_retention.into(),
            gateway_config_pda,
        )
        .unwrap();
        self.fixture.send_tx(&[ix]).await?;

        let account = self
            .fixture
            .banks_client
            .get_account(gateway_config_pda)
            .await
            .unwrap()
            .expect("metadata");

        assert_eq!(
            account.owner,
            axelar_solana_gateway::id(),
            "gateway config account must be owned by the gateway program "
        );

        Ok(gateway_config_pda)
    }

    /// Initialise a new payload verification session
    pub async fn initialize_payload_verification_session(
        &mut self,
        execute_data: &ExecuteData,
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        let ix = axelar_solana_gateway::instructions::initialize_payload_verification_session(
            self.payer.pubkey(),
            self.gateway_root_pda,
            execute_data.payload_merkle_root,
        )
        .unwrap();
        self.fixture.send_tx(&[ix]).await
    }

    /// Initialsie a new payload verification session and sign using the
    /// provided verifier set
    pub async fn init_payload_session_and_verify(
        &mut self,
        execute_data: &ExecuteData,
    ) -> Result<Pubkey, BanksTransactionResultWithMetadata> {
        let gateway_config_pda = get_gateway_root_config_pda().0;
        self.initialize_payload_verification_session(execute_data)
            .await?;
        let (verifier_set_tracker_pda, _verifier_set_tracker_bump) =
            get_verifier_set_tracker_pda(execute_data.signing_verifier_set_merkle_root);

        for signature_leaves in &execute_data.signing_verifier_set_leaves {
            // Verify the signature
            let ix = axelar_solana_gateway::instructions::verify_signature(
                gateway_config_pda,
                verifier_set_tracker_pda,
                execute_data.payload_merkle_root,
                signature_leaves.clone(),
            )
            .unwrap();
            let tx_result = self
                .send_tx(&[
                    ComputeBudgetInstruction::set_compute_unit_limit(250_000),
                    ix,
                ])
                .await?;
            tx_result.result.unwrap();
        }

        // Check that the PDA contains the expected data
        let (verification_pda, _bump) = axelar_solana_gateway::get_signature_verification_pda(
            &gateway_config_pda,
            &execute_data.payload_merkle_root,
        );
        Ok(verification_pda)
    }

    /// Create a signing session and approve all the messages that have been
    /// provided
    #[allow(clippy::unreachable)]
    pub async fn sign_session_and_approve_messages(
        &mut self,
        signers: &SigningVerifierSet,
        messages: &[Message],
    ) -> Result<Vec<MerkleisedMessage>, BanksTransactionResultWithMetadata> {
        let payload = Payload::Messages(Messages(messages.to_vec()));
        let execute_data = self.construct_execute_data(signers, payload);
        let verification_session_pda = self.init_payload_session_and_verify(&execute_data).await?;

        let MerkleisedPayload::NewMessages { messages } = execute_data.payload_items else {
            unreachable!("we constructed a message batch");
        };

        for message_info in &messages {
            self.approve_message(
                execute_data.payload_merkle_root,
                message_info.clone(),
                verification_session_pda,
            )
            .await?;
        }
        Ok(messages)
    }

    /// Construct new [`ExecuteData`] by signing the data and generading all the
    /// stuff that needs to be encoded.
    pub fn construct_execute_data(
        &mut self,
        signers: &SigningVerifierSet,
        payload: Payload,
    ) -> ExecuteData {
        let vs = signers.verifier_set();
        self.construct_execute_data_with_custom_verifier_set(signers, &vs, payload)
    }

    /// Construct new [`ExecuteData`] by signing the data and generading all the
    /// stuff that needs to be encoded.
    ///
    /// The function will use the provided `verifier_set` for encoding, and the
    /// `signers` for signing the data.
    pub fn construct_execute_data_with_custom_verifier_set(
        &mut self,
        signers: &SigningVerifierSet,
        verifier_set: &VerifierSet,
        payload: Payload,
    ) -> ExecuteData {
        let payload_hash =
            hash_payload(&self.domain_separator, verifier_set, payload.clone()).unwrap();
        let signatures = {
            signers
                .signers
                .iter()
                .map(|signer| {
                    let signature = signer.secret_key.sign(&payload_hash);
                    (signer.public_key, signature)
                })
                .collect()
        };
        let execute_data = axelar_solana_encoding::encode(
            verifier_set,
            &signatures,
            self.domain_separator,
            payload,
        )
        .unwrap();

        borsh::from_slice::<ExecuteData>(&execute_data).unwrap()
    }

    /// Approve a single message on the Gateway
    pub async fn approve_message(
        &mut self,
        payload_merkle_root: [u8; 32],
        message: MerkleisedMessage,
        verification_session_pda: Pubkey,
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        let command_id = command_id(
            &message.leaf.message.cc_id.chain,
            &message.leaf.message.cc_id.id,
        );

        let (incoming_message_pda, incoming_message_pda_bump) =
            get_incoming_message_pda(&command_id);

        let ix = axelar_solana_gateway::instructions::approve_messages(
            message,
            payload_merkle_root,
            self.gateway_root_pda,
            self.payer.pubkey(),
            verification_session_pda,
            incoming_message_pda,
            incoming_message_pda_bump,
        )
        .unwrap();
        self.send_tx(&[ix]).await
    }

    /// Start a new payload verification session for signer rotation, and rotate
    /// the signers.
    pub async fn sign_session_and_rotate_signers(
        &mut self,
        signers: &SigningVerifierSet,
        new_verifier_set: &VerifierSet,
    ) -> Result<
        (
            Pubkey,
            Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata>,
        ),
        BanksTransactionResultWithMetadata,
    > {
        let payload = Payload::NewVerifierSet(new_verifier_set.clone());
        let execute_data = self.construct_execute_data(signers, payload);
        let verification_session_account =
            self.init_payload_session_and_verify(&execute_data).await?;

        let res = self
            .rotate_signers(signers, new_verifier_set, verification_session_account)
            .await;
        Ok((verification_session_account, res))
    }

    /// Rotate the signers.
    /// The assumption is that the signer verification session is already
    /// complete beforehand.
    pub async fn rotate_signers(
        &mut self,
        signers: &SigningVerifierSet,
        new_verifier_set: &VerifierSet,
        verification_session_account: Pubkey,
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        let new_verifier_set_hash =
            verifier_set_hash::<NativeHasher>(new_verifier_set, &self.domain_separator).unwrap();
        let gateway_config_pda = get_gateway_root_config_pda().0;
        let (new_vs_tracker_pda, new_vs_tracker_bump) =
            axelar_solana_gateway::get_verifier_set_tracker_pda(new_verifier_set_hash);
        let rotate_signers_ix = axelar_solana_gateway::instructions::rotate_signers(
            gateway_config_pda,
            verification_session_account,
            signers.verifier_set_tracker().0,
            new_vs_tracker_pda,
            self.payer.pubkey(),
            None,
            new_verifier_set_hash,
            new_vs_tracker_bump,
        )
        .unwrap();

        self.send_tx(&[rotate_signers_ix]).await
    }

    /// Call `execute` on an axelar-executable program
    pub async fn execute_on_axelar_executable(
        &mut self,
        message: Message,
        raw_payload: &[u8],
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        let (incoming_message_pda, _bump) =
            get_incoming_message_pda(&command_id(&message.cc_id.chain, &message.cc_id.id));
        let ix = axelar_executable::construct_axelar_executable_ix(
            message,
            raw_payload,
            incoming_message_pda,
        )
        .unwrap();
        self.send_tx(&[ix]).await
    }

    /// Get the signature verification session data (deserialised)
    pub async fn signature_verification_session(
        &mut self,
        verification_pda: Pubkey,
    ) -> SignatureVerificationSessionData {
        let verification_session_account = self
            .banks_client
            .get_account(verification_pda)
            .await
            .ok()
            .flatten()
            .expect("verification session PDA account should exist");

        assert_eq!(
            verification_session_account.owner,
            axelar_solana_gateway::ID,
            "verification session must be owned by the gateway"
        );
        let mut buffer = [0_u8; SignatureVerificationSessionData::LEN];
        buffer.copy_from_slice(verification_session_account.data());

        bytemuck::cast(buffer)
    }

    /// Get the gateway root config data
    pub async fn gateway_confg(&mut self, gateway_root_pda: Pubkey) -> GatewayConfig {
        self.banks_client
            .get_account_data_with_borsh(gateway_root_pda)
            .await
            .unwrap()
    }

    /// Get the verifier set tracker data
    pub async fn verifier_set_tracker(
        &mut self,
        verifiers_set_tracker_pda: Pubkey,
    ) -> VerifierSetTracker {
        self.banks_client
            .get_account_data_with_borsh(verifiers_set_tracker_pda)
            .await
            .expect("could not get the account & deserialise it")
    }

    /// Get the verifier set tracker data
    pub async fn incoming_message(
        &mut self,
        incoming_message_pda: Pubkey,
    ) -> IncomingMessageWrapper {
        let pda = self
            .banks_client
            .get_account(incoming_message_pda)
            .await
            .ok()
            .flatten()
            .expect("PDA account should exist");

        assert_eq!(
            pda.owner,
            axelar_solana_gateway::ID,
            "must be owned by the gateway"
        );
        let mut buffer = [0_u8; IncomingMessageWrapper::LEN];
        buffer.copy_from_slice(pda.data());

        bytemuck::cast(buffer)
    }
}

/// Test fixture builder for the Solana Axelar Gateway integration
#[derive(Debug, typed_builder::TypedBuilder)]
pub struct SolanaAxelarIntegration {
    #[builder(default)]
    initial_signer_weights: Vec<u128>,
    #[builder(default, setter(strip_option))]
    custom_quorum: Option<u128>,
    #[builder(default)]
    minimum_rotate_signers_delay_seconds: u64,
    #[builder(default = [42; 32])]
    domain_separator: [u8; 32],
    #[builder(default = 333)]
    initial_nonce: u64,
    #[builder(default = 1)]
    previous_signers_retention: u64,
    #[builder(default)]
    /// Extra programs (besides the Solana gateway) that we need to deploy
    /// The parameters -- name of the program .so file (with the extensoin) and
    /// the program id
    ///
    /// ```ignore
    /// vec![("gmp_gatefay.so".into(), gmp_gateway::id())]
    /// ```
    programs_to_deploy: Vec<(PathBuf, Pubkey)>,
}

impl SolanaAxelarIntegration {
    /// Setup a new Axelar Solana Gateway without instaintiating the root config
    #[allow(clippy::unwrap_used)]
    pub async fn setup_without_init_config(self) -> SolanaAxelarIntegrationMetadata {
        // Create a new ProgramTest instance
        let mut fixture = TestFixture::new(ProgramTest::default()).await;
        // Generate a new keypair for the upgrade authority
        let upgrade_authority = Keypair::new();

        // deploy non-gateway programs
        for (program_name, program_id) in self.programs_to_deploy {
            let program_bytecode_path = workspace_root_dir()
                .join("target")
                .join("deploy")
                .join(program_name);
            dbg!(&program_bytecode_path);
            let program_bytecode = tokio::fs::read(&program_bytecode_path).await.unwrap();
            fixture
                .register_upgradeable_program(
                    &program_bytecode,
                    &upgrade_authority.pubkey(),
                    &program_id,
                )
                .await;
        }

        // deploy solana gateway
        let gateway_program_bytecode =
            tokio::fs::read("../../target/deploy/axelar_solana_gateway.so")
                .await
                .unwrap();
        fixture
            .register_upgradeable_program(
                &gateway_program_bytecode,
                &upgrade_authority.pubkey(),
                &axelar_solana_gateway::id(),
            )
            .await;
        let operator = Keypair::new();
        let initial_signers = make_verifiers_with_quorum(
            &self.initial_signer_weights,
            self.initial_nonce,
            self.custom_quorum
                .unwrap_or_else(|| self.initial_signer_weights.iter().sum()),
            self.domain_separator,
        );
        SolanaAxelarIntegrationMetadata {
            domain_separator: self.domain_separator,
            upgrade_authority,
            fixture,
            signers: initial_signers,
            gateway_root_pda: axelar_solana_gateway::get_gateway_root_config_pda().0,
            operator,
            previous_signers_retention: self.previous_signers_retention,
            minimum_rotate_signers_delay_seconds: self.minimum_rotate_signers_delay_seconds,
        }
    }

    /// Setup a new Axelar Solana Gateway integration.
    /// This method also initialises the Gateway config.
    pub async fn setup(self) -> SolanaAxelarIntegrationMetadata {
        let mut metadata = self.setup_without_init_config().await;
        let _gateway_root_pda = metadata.initialize_gateway_config_account().await;
        metadata
    }
}

/// Get events emitted by the Gateway
pub fn get_gateway_events(
    tx: &solana_program_test::BanksTransactionResultWithMetadata,
) -> Vec<EventContainer> {
    tx.metadata
        .as_ref()
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GatewayEvent::parse_log)
        .collect::<Vec<_>>()
}

/// Create a new verifier set
pub fn make_verifier_set(
    weights: &[u128],
    nonce: u64,
    domain_separator: [u8; 32],
) -> SigningVerifierSet {
    let signers = weights
        .iter()
        .copied()
        .map(create_signer_with_weight)
        .collect::<Vec<_>>();
    let signers = Arc::from(signers);

    SigningVerifierSet::new(signers, nonce, domain_separator)
}

/// Create a new verifier set with a custom quorum
pub fn make_verifiers_with_quorum(
    weights: &[u128],
    nonce: u64,
    quorum: u128,
    domain_separator: [u8; 32],
) -> SigningVerifierSet {
    let signers = weights
        .iter()
        .copied()
        .map(create_signer_with_weight)
        .collect::<Vec<_>>();
    let signers = Arc::from(signers);

    SigningVerifierSet::new_with_quorum(signers, nonce, quorum, domain_separator)
}

/// Make new random messages
#[must_use]
pub fn make_messages(num_messages: usize) -> Vec<Message> {
    (0..num_messages).map(|_| random_message()).collect()
}

/// Random GMP Message
#[must_use]
pub fn random_message() -> Message {
    Message {
        cc_id: CrossChainId {
            chain: random_chain_name(),
            id: random_string(20),
        },
        source_address: generate_random_hex_address(),
        destination_chain: random_chain_name(),
        destination_address: generate_random_hex_address(),
        payload_hash: random_bytes::<32>(),
    }
}

#[allow(clippy::indexing_slicing)]
fn random_chain_name() -> String {
    let chains = ["Ethereum", "Solana", "Polkadot", "Binance Smart Chain"];
    let mut rng = rand::thread_rng();
    chains[rng.gen_range(0..chains.len())].to_owned()
}

/// New random HEX address (e.g. ethereum address)
/// It's not guaranteed to be a valid address
#[must_use]
#[allow(clippy::unwrap_used)]
pub fn generate_random_hex_address() -> String {
    let mut rng = rand::thread_rng();

    (0_u8..40_u8) // 40 characters for a 20-byte address in hex
        .fold(String::new(), |mut output, _| {
            write!(&mut output, "{:x}", rng.gen_range(0..16_u8)).unwrap();
            output
        })
}

/// Random bytes
#[must_use]
pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0_u8; N];
    rand::rngs::OsRng.fill(&mut bytes[..]);
    bytes
}

/// Random string
pub fn random_string(len: usize) -> String {
    rand::rngs::OsRng
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
