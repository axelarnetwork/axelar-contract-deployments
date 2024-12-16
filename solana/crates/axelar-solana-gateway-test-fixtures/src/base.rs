//! Module that contains the base test fixtures

use std::path::PathBuf;

use solana_program::hash::Hash;
use solana_program::pubkey::Pubkey;
use solana_program_test::{
    BanksClient, BanksClientError, BanksTransactionResultWithMetadata, ProgramTest,
    ProgramTestBanksClientExt as _, ProgramTestContext,
};
use solana_sdk::account::{Account, AccountSharedData, WritableAccount as _};
use solana_sdk::account_utils::StateMut as _;
use solana_sdk::bpf_loader_upgradeable::{self, UpgradeableLoaderState};
use solana_sdk::clock::Clock;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer as _;
use solana_sdk::signers::Signers;
use solana_sdk::system_instruction;
use solana_sdk::transaction::Transaction;

/// Base test fixture wrapper that's agnostic to the Axelar Solana Gateway, it
/// also provides useful utilities.
pub struct TestFixture {
    /// The base test program context
    pub context: ProgramTestContext,
    /// inner banks client
    pub banks_client: BanksClient,
    /// The account that signs all transactions by default
    pub payer: Keypair,
    /// Recent blockhash
    pub recent_blockhash: Hash,
}

impl TestFixture {
    /// Create a new test fixture
    pub async fn new(pt: ProgramTest) -> Self {
        let context = pt.start_with_context().await;
        Self {
            banks_client: context.banks_client.clone(),
            payer: context.payer.insecure_clone(),
            recent_blockhash: context.last_blockhash,
            context,
        }
    }

    /// Refresh the latest blockhash
    pub async fn refresh_blockhash(&mut self) -> Hash {
        self.recent_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&self.recent_blockhash)
            .await
            .unwrap();
        self.recent_blockhash
    }

    /// Forward the time
    pub async fn forward_time(&mut self, add_time: i64) {
        // get clock sysvar
        let clock_sysvar: Clock = self.banks_client.get_sysvar().await.unwrap();

        // update clock
        let mut new_clock = clock_sysvar;
        new_clock.unix_timestamp = new_clock.unix_timestamp.saturating_add(add_time);

        // set clock
        self.context.set_sysvar(&new_clock);
    }

    /// Send a new transaction.
    /// Using the default `self.payer` for signing.
    pub async fn send_tx(
        &mut self,
        ixs: &[Instruction],
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        self.send_tx_with_custom_signers(ixs, &[&self.payer.insecure_clone()])
            .await
    }

    /// Send a new transaction while also providing the signers to use
    pub async fn send_tx_with_custom_signers<T: Signers + ?Sized>(
        &mut self,
        ixs: &[Instruction],
        signing_keypairs: &T,
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
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

        if tx.result.is_ok() {
            return Ok(tx);
        }
        Err(tx)
    }

    /// Go through all the steps of deploying an upgradeable solana program -
    /// the same way that's described in [Solana docs](https://solana.com/docs/programs/deploying#state-accounts)
    ///
    /// Although this is not practical for our tests because we cannot ensure
    /// that the program id does not change. Currently this is used in a
    /// test to assert that the programdata accounts get initialized the same
    /// way when manually registering the state accounts.
    /// See `test_manually_added_bpf_upgradeable_accounts_contain_expected_state` test for usage.
    ///
    /// For in-test usage see `register_upgradeable_program` method instead.
    #[deprecated = "use `register_upgradeable_program`"]
    pub async fn deploy_upgradeable_program(
        &mut self,
        program_bytecode: &[u8],
        upgrade_authority: &Keypair,
        program_keypair: &Keypair,
    ) -> Pubkey {
        let buffer_keypair = Keypair::new();
        let buffer_pubkey = buffer_keypair.pubkey();
        let program_address = program_keypair.pubkey();
        let (program_data_pda, _) = Pubkey::find_program_address(
            &[program_address.as_ref()],
            &bpf_loader_upgradeable::id(),
        );
        let program_bytecode_size =
            UpgradeableLoaderState::size_of_programdata(program_bytecode.len());
        let rent = self
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(program_bytecode_size)
            // for some reason without this we get an error
            .saturating_mul(2);

        let fee_payer_signer = self.payer.insecure_clone();

        let ixs = bpf_loader_upgradeable::create_buffer(
            &fee_payer_signer.pubkey(),
            &buffer_pubkey,
            &upgrade_authority.pubkey(),
            rent,
            program_bytecode.len(),
        )
        .unwrap();
        self.send_tx_with_custom_signers(&ixs, &[&self.payer.insecure_clone(), &buffer_keypair])
            .await
            .unwrap();
        let chunk_size = 1024; // Adjust the chunk size as needed

        let mut offset = 0;
        for chunk in program_bytecode.chunks(chunk_size) {
            let write_ix = bpf_loader_upgradeable::write(
                &buffer_pubkey,
                &upgrade_authority.pubkey(),
                offset,
                chunk.to_vec(),
            );
            self.send_tx_with_custom_signers(
                &[write_ix],
                &[&self.payer.insecure_clone(), upgrade_authority],
            )
            .await
            .unwrap();

            offset = offset.saturating_add(u32::try_from(chunk.len()).unwrap());
        }

        let deploy_ix = bpf_loader_upgradeable::deploy_with_max_program_len(
            &self.payer.pubkey(),
            &program_address,
            &buffer_pubkey,
            &upgrade_authority.pubkey(),
            rent,
            program_bytecode.len(),
        )
        .unwrap();
        self.send_tx_with_custom_signers(
            &deploy_ix,
            &[
                &self.payer.insecure_clone(),
                program_keypair,
                upgrade_authority,
            ],
        )
        .await
        .unwrap();
        program_data_pda
    }

    /// Register the necessary `bpf_loader_upgradeable` PDAs for a given program
    /// bytecode to ensure that the program is upgradable.
    /// This feature is not provided by the `solana_program_test` [see this github issue](https://github.com/solana-labs/solana/issues/22950) - we could create a pr and upstream the changes
    pub async fn register_upgradeable_program(
        &mut self,
        program_bytecode: &[u8],
        upgrade_authority: &Pubkey,
        program_keypair: &Pubkey,
    ) -> Pubkey {
        let (program_data_pda, _) = Pubkey::find_program_address(
            &[program_keypair.as_ref()],
            &bpf_loader_upgradeable::id(),
        );

        add_upgradeable_loader_account(
            &mut self.context,
            program_keypair,
            &UpgradeableLoaderState::Program {
                programdata_address: program_data_pda,
            },
            UpgradeableLoaderState::size_of_program(),
            |acc| acc.set_executable(true),
        )
        .await;
        let programdata_data_offset = UpgradeableLoaderState::size_of_programdata_metadata();
        let program_data_len = program_bytecode
            .len()
            .saturating_add(programdata_data_offset);

        add_upgradeable_loader_account(
            &mut self.context,
            &program_data_pda,
            &UpgradeableLoaderState::ProgramData {
                slot: 0,
                upgrade_authority_address: Some(*upgrade_authority),
            },
            program_data_len,
            |account| {
                account
                    .data_as_mut_slice()
                    .get_mut(programdata_data_offset..)
                    .unwrap()
                    .copy_from_slice(program_bytecode);
            },
        )
        .await;

        program_data_pda
    }

    /// Get the account data
    ///
    /// # Panics
    /// if the account does not exist or the expected owner does not match.
    #[allow(clippy::panic)]
    pub async fn get_account(&mut self, account: &Pubkey, expected_owner: &Pubkey) -> Account {
        match self.try_get_account(account, expected_owner).await {
            Ok(Some(account)) => account,
            Ok(None) => panic!("account not found"),
            Err(error) => panic!("error while getting account: {error}"),
        }
    }

    /// Tries to get an account.
    ///
    /// Non-panicking version of `Self::get_account`
    pub async fn try_get_account(
        &mut self,
        account: &Pubkey,
        expected_owner: &Pubkey,
    ) -> Result<Option<Account>, BanksClientError> {
        match self.banks_client.get_account(*account).await? {
            None => Ok(None),
            Some(account) if account.owner == *expected_owner => Ok(Some(account)),
            Some(_) => Err(BanksClientError::ClientError("unexpected account owner")),
        }
    }

    /// Funds the account using the `self.payer` as the bank
    pub async fn fund_account(&mut self, to: &Pubkey, amount: u64) {
        let from = self.payer.pubkey();
        let ix = system_instruction::transfer(&from, to, amount);
        self.send_tx(&[ix]).await.expect("failed to fund account");
    }
}

/// Utility triat to find a specific log within the
/// [`BanksTransactionResultWithMetadata`] type
pub trait FindLog {
    /// Find the desired log
    fn find_log(&self, expected: &str) -> Option<&str>;
}

impl FindLog for BanksTransactionResultWithMetadata {
    fn find_log(&self, expected: &str) -> Option<&str> {
        self.metadata.as_ref().and_then(|x| {
            x.log_messages
                .iter()
                .find(|log| log.contains(expected))
                .map(std::string::String::as_str)
        })
    }
}

pub(crate) async fn add_upgradeable_loader_account(
    context: &mut ProgramTestContext,
    account_address: &Pubkey,
    account_state: &UpgradeableLoaderState,
    account_data_len: usize,
    account_callback: impl Fn(&mut AccountSharedData),
) {
    let rent = context.banks_client.get_rent().await.unwrap();
    let mut account = AccountSharedData::new(
        rent.minimum_balance(account_data_len),
        account_data_len,
        &bpf_loader_upgradeable::id(),
    );
    account
        .set_state(account_state)
        .expect("state failed to serialize into account data");
    account_callback(&mut account);
    context.set_account(account_address, &account);
}

#[must_use]
pub(crate) fn workspace_root_dir() -> PathBuf {
    let dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    PathBuf::from(dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

#[cfg(test)]
mod tests {

    use solana_sdk::account::ReadableAccount as _;

    use super::*;

    /// Try to deploy the same program elf file using the
    /// `bbf_loader_upgradeable` program directly, and regiestering the PDAs
    /// manually.
    /// The core asserts is to ensure that the account data storage
    /// is the same, thus ensuring that both operatiosn are somewhat equivalent.
    #[tokio::test]
    async fn test_manually_added_bpf_upgradeable_accounts_contain_expected_state() {
        // setup
        let mut test_fixture = TestFixture::new(ProgramTest::default()).await;
        // source: https://github.com/solana-labs/solana/blob/master/cli/tests/fixtures/noop.so?plain=1#L1
        let noop_so = include_bytes!("../noop.so");
        let upgrade_authority = Keypair::new();
        let program_keypair = Keypair::new();
        let program_keypair_2 = Keypair::new();

        // Action
        #[allow(deprecated)]
        let programdata_pda = test_fixture
            .deploy_upgradeable_program(noop_so, &upgrade_authority, &program_keypair)
            .await;
        let programdata_pda_2 = test_fixture
            .register_upgradeable_program(
                noop_so,
                &upgrade_authority.pubkey(),
                &program_keypair_2.pubkey(),
            )
            .await;

        // Assert - program_id gets initialised
        let program_id_account = get_account(&mut test_fixture, program_keypair.pubkey()).await;
        let program_id_account_2 = get_account(&mut test_fixture, program_keypair_2.pubkey()).await;
        let loader_state =
            bincode::deserialize::<UpgradeableLoaderState>(program_id_account.data()).unwrap();
        let loader_state_2 =
            bincode::deserialize::<UpgradeableLoaderState>(program_id_account_2.data()).unwrap();
        assert!(matches!(
            loader_state,
            UpgradeableLoaderState::Program {
                programdata_address
            }
            if programdata_address == programdata_pda
        ));
        assert!(matches!(
            loader_state_2,
            UpgradeableLoaderState::Program {
                programdata_address
            } if programdata_address == programdata_pda_2
        ));

        // Assert - programdata gets initialised
        let programdata_account = get_account(&mut test_fixture, programdata_pda).await;
        let programdata_account_2 = get_account(&mut test_fixture, programdata_pda_2).await;
        let loader_state = bincode::deserialize::<UpgradeableLoaderState>(
            programdata_account
                .data()
                .get(0..UpgradeableLoaderState::size_of_programdata_metadata())
                .unwrap(),
        )
        .unwrap();
        let loader_state_2 = bincode::deserialize::<UpgradeableLoaderState>(
            programdata_account_2
                .data()
                .get(0..UpgradeableLoaderState::size_of_programdata_metadata())
                .unwrap(),
        )
        .unwrap();
        let expected_upgrade_authority_address = upgrade_authority.pubkey();
        assert!(matches!(
            loader_state,
            UpgradeableLoaderState::ProgramData {
                upgrade_authority_address,
                ..
            } if upgrade_authority_address == Some(expected_upgrade_authority_address)
        ));
        assert!(matches!(
            loader_state_2,
            UpgradeableLoaderState::ProgramData {
                upgrade_authority_address,
                ..
            } if upgrade_authority_address == Some(expected_upgrade_authority_address)
        ));
        assert_eq!(
            programdata_account_2.data().len(),
            UpgradeableLoaderState::size_of_programdata_metadata() + noop_so.len()
        );
        assert_eq!(
            programdata_account.data().len(),
            programdata_account_2.data().len()
        );
    }

    async fn get_account(test_fixture: &mut TestFixture, address: Pubkey) -> Account {
        test_fixture
            .banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap()
    }
}
