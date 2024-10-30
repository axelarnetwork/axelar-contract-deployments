use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use axelar_solana_gateway::instructions::InitializeConfig;
use axelar_solana_gateway::state::verifier_set_tracker::VerifierSetHash;
use borsh::BorshDeserialize;
use solana_program::hash::Hash;
use solana_program_test::tokio::time;
use solana_program_test::{
    processor, BanksTransactionResultWithMetadata, ProgramTest, ProgramTestBanksClientExt,
    ProgramTestContext,
};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::signers::Signers;
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;

pub struct TestRunner {
    program_test_context: ProgramTestContext,
}

static COOLDOWN: OnceLock<u64> = OnceLock::new();

impl TestRunner {
    pub async fn new() -> Self {
        let program_test_context = ProgramTest::new(
            "axelar_solana_gateway",
            axelar_solana_gateway::ID,
            processor!(axelar_solana_gateway::entrypoint::process_instruction),
        )
        .start_with_context()
        .await;
        Self {
            program_test_context,
        }
    }

    pub async fn get_account<T: solana_program::program_pack::Pack + BorshDeserialize>(
        &mut self,
        account: &Pubkey,
        expected_owner: &Pubkey,
    ) -> T {
        let account = self
            .banks_client
            .get_account(*account)
            .await
            .expect("get_account")
            .expect("account not none");
        account.check_initialized_pda::<T>(expected_owner).unwrap()
    }

    pub async fn refresh_blockhash(&mut self) -> Hash {
        let last_blockhash = self.last_blockhash;
        self.banks_client
            .get_new_latest_blockhash(&last_blockhash)
            .await
            .unwrap()
    }

    pub async fn send_tx(&mut self, ixs: &[Instruction]) {
        self.send_tx_with_custom_signers(ixs, &[&self.payer.insecure_clone()])
            .await;
    }

    pub async fn send_tx_with_custom_signers<T: Signers + ?Sized>(
        &mut self,
        ixs: &[Instruction],
        signing_keypairs: &T,
    ) {
        let tx = self
            .send_tx_with_custom_signers_with_metadata(ixs, signing_keypairs)
            .await;
        assert!(tx.result.is_ok());
    }

    pub async fn send_tx_with_custom_signers_with_metadata<T: Signers + ?Sized>(
        &mut self,
        ixs: &[Instruction],
        signing_keypairs: &T,
    ) -> BanksTransactionResultWithMetadata {
        let hash = self.refresh_blockhash().await;
        let tx = Transaction::new_signed_with_payer(
            ixs,
            Some(&self.payer.pubkey()),
            signing_keypairs,
            hash,
        );
        let tx = self
            .banks_client
            .process_transaction_with_metadata(tx)
            .await
            .unwrap();

        // Sleep to allow the transaction to be processed.
        //
        // The solana test program otherwise can't keep up with the
        // speed of the transactions for some more intense tests on
        // weaker CI machines
        time::sleep(time::Duration::from_millis(*COOLDOWN.get_or_init(|| {
            if std::env::var("CI").is_ok() {
                200
            } else {
                50
            }
        })))
        .await;

        tx
    }

    pub async fn send_tx_with_metadata(
        &mut self,
        ixs: &[Instruction],
    ) -> BanksTransactionResultWithMetadata {
        self.send_tx_with_custom_signers_with_metadata(ixs, &[&self.payer.insecure_clone()])
            .await
    }

    pub async fn initialize_gateway_config_account(
        &mut self,
        init_config: InitializeConfig<VerifierSetHash>,
    ) -> Pubkey {
        let (gateway_config_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
        let ix = axelar_solana_gateway::instructions::initialize_config(
            self.payer.pubkey(),
            init_config.clone(),
            gateway_config_pda,
        )
        .unwrap();
        self.send_tx(&[ix]).await;

        let account = self
            .banks_client
            .get_account(gateway_config_pda)
            .await
            .unwrap()
            .expect("metadata");

        assert_eq!(account.owner, axelar_solana_gateway::id());

        gateway_config_pda
    }

    pub async fn initialize_payload_verification_session(
        &mut self,
        gateway_config_pda: Pubkey,
        payload_merkle_root: [u8; 32],
    ) {
        let ix = axelar_solana_gateway::instructions::initialize_payload_verification_session(
            self.payer.pubkey(),
            gateway_config_pda,
            payload_merkle_root,
        )
        .unwrap();
        let tx_result = self.send_tx_with_metadata(&[ix]).await;
        assert!(tx_result.result.is_ok());
    }
}

impl Deref for TestRunner {
    type Target = ProgramTestContext;

    fn deref(&self) -> &Self::Target {
        &self.program_test_context
    }
}
impl DerefMut for TestRunner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.program_test_context
    }
}
