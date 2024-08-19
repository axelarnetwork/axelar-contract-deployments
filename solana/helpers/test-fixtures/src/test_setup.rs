use std::collections::BTreeMap;

use axelar_message_primitives::command::U256;
use axelar_message_primitives::DataPayload;
use axelar_rkyv_encoding::types::u128::U128;
use axelar_rkyv_encoding::types::{ArchivedExecuteData, Message, Payload, VerifierSet};
use borsh::BorshDeserialize;
use gateway::commands::OwnedCommand;
use gateway::instructions::{InitializeConfig, VerifierSetWraper};
use gateway::state::{GatewayApprovedCommand, GatewayExecuteData};
use itertools::Itertools;
use solana_program::hash::Hash;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_program_test::{
    BanksClient, BanksTransactionResultWithMetadata, ProgramTest, ProgramTestBanksClientExt,
    ProgramTestContext,
};
use solana_sdk::account::{AccountSharedData, ReadableAccount, WritableAccount};
use solana_sdk::account_utils::StateMut;
use solana_sdk::bpf_loader_upgradeable::{self, UpgradeableLoaderState};
use solana_sdk::clock::Clock;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::signers::Signers;
use solana_sdk::transaction::Transaction;
use spl_token::state::Mint;
pub use {connection_router, interchain_token_transfer_gmp};

use crate::account::CheckValidPDAInTests;
use crate::execute_data::prepare_execute_data;
use crate::test_signer::TestSigner;

const DOMAIN_SEPARATOR: [u8; 32] = [42u8; 32];

pub struct TestFixture {
    pub context: ProgramTestContext,
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub recent_blockhash: Hash,
    pub domain_separator: [u8; 32],
}

impl TestFixture {
    pub async fn new(pt: ProgramTest) -> TestFixture {
        let context = pt.start_with_context().await;
        TestFixture {
            banks_client: context.banks_client.clone(),
            payer: context.payer.insecure_clone(),
            recent_blockhash: context.last_blockhash,
            context,
            domain_separator: DOMAIN_SEPARATOR,
        }
    }

    pub async fn refresh_blockhash(&mut self) -> Hash {
        self.recent_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&self.recent_blockhash)
            .await
            .unwrap();
        self.recent_blockhash
    }

    pub async fn forward_time(&mut self, add_time: i64) {
        // get clock sysvar
        let clock_sysvar: Clock = self.banks_client.get_sysvar().await.unwrap();

        // update clock
        let mut new_clock = clock_sysvar.clone();
        new_clock.unix_timestamp += add_time;

        // set clock
        self.context.set_sysvar(&new_clock)
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

        // make everything slower on CI to prevent flaky tests
        if std::env::var("CI").is_ok() {
            // sleep for 200 millis to allow the transaction to be processed. The solana
            // test program otherwise can't keep up with the speed of the transactions for
            // some more intense tests on weaker CI machines
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        tx
    }

    pub async fn send_tx_with_metadata(
        &mut self,
        ixs: &[Instruction],
    ) -> BanksTransactionResultWithMetadata {
        self.send_tx_with_custom_signers_with_metadata(ixs, &[&self.payer.insecure_clone()])
            .await
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
            * 2; // for some reason without this we get an error

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
            .await;
        let chunk_size = 1024; // Adjust the chunk size as needed

        let mut offset = 0;
        for chunk in program_bytecode.chunks(chunk_size) {
            println!("writing to buffer");
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
            .await;

            offset += chunk.len() as u32;
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
        .await;
        program_data_pda
    }

    /// Register the necessary bpf_loader_upgradeable PDAs for a given program
    /// bytecode to ensure that the program is upgradable.
    /// This feature is not provided by the solana_program_test crate - https://github.com/solana-labs/solana/issues/22950 - we could create a pr and upstream the changes
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
            |_| {},
        )
        .await;
        let programdata_data_offset = UpgradeableLoaderState::size_of_programdata_metadata();
        let program_data_len = program_bytecode.len() + programdata_data_offset;
        println!("program data len {program_data_len}");
        add_upgradeable_loader_account(
            &mut self.context,
            &program_data_pda,
            &UpgradeableLoaderState::ProgramData {
                slot: 0,
                upgrade_authority_address: Some(*upgrade_authority),
            },
            program_data_len,
            |account| {
                account.data_as_mut_slice()[programdata_data_offset..]
                    .copy_from_slice(program_bytecode)
            },
        )
        .await;

        program_data_pda
    }

    pub async fn init_gas_service(&mut self) -> Pubkey {
        let (root_pda_address, _) = gas_service::get_gas_service_root_pda();
        let ix =
            gas_service::instruction::create_initialize_root_pda_ix(self.payer.pubkey()).unwrap();
        self.send_tx(&[ix]).await;

        let root_pda_data = self
            .banks_client
            .get_account(root_pda_address)
            .await
            .unwrap()
            .unwrap();
        let root_pda_data =
            gas_service::accounts::GasServiceRootPDA::try_from_slice(root_pda_data.data.as_slice())
                .unwrap();

        assert!(root_pda_data.check_authority(self.payer.pubkey()));

        root_pda_address
    }

    pub fn create_verifier_set(&self, signers: &[TestSigner], nonce: u64) -> VerifierSetWraper {
        let threshold = signers
            .iter()
            .map(|s| s.weight)
            .try_fold(U128::ZERO, U128::checked_add)
            .expect("no arithmetic overflow");

        self.create_verifier_set_with_custom_params(signers, threshold, nonce)
    }

    pub fn create_verifier_sets(&self, signers: &[(&[TestSigner], u64)]) -> Vec<VerifierSetWraper> {
        signers
            .iter()
            .map(|(signers, nonce)| self.create_verifier_set(signers, *nonce))
            .collect_vec()
    }

    pub fn create_verifier_sets_with_thershold(
        &self,
        signers: &[(&[TestSigner], u64, U128)],
    ) -> Vec<VerifierSetWraper> {
        signers
            .iter()
            .map(|(signers, nonce, threshold)| {
                self.create_verifier_set_with_custom_params(signers, *threshold, *nonce)
            })
            .collect_vec()
    }

    pub fn create_verifier_set_with_custom_params(
        &self,
        signers: &[TestSigner],
        threshold: U128,
        nonce: u64,
    ) -> VerifierSetWraper {
        let signers: BTreeMap<_, _> = signers.iter().map(|s| (s.public_key, s.weight)).collect();
        let verifier_set = VerifierSet::new(nonce, signers, threshold);
        VerifierSetWraper::new_from_verifier_set(verifier_set).unwrap()
    }

    pub fn base_initialize_config(&self) -> InitializeConfig {
        InitializeConfig {
            domain_separator: self.domain_separator,
            initial_signer_sets: vec![],
            minimum_rotation_delay: 0,
            operator: Pubkey::new_unique(),
            previous_signers_retention: U256::from(0_u128),
        }
    }

    pub async fn initialize_gateway_config_account(
        &mut self,
        init_config: InitializeConfig,
    ) -> Pubkey {
        let (gateway_config_pda, _) = gateway::get_gateway_root_config_pda();
        let ix = gateway::instructions::initialize_config(
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

        assert_eq!(account.owner, gateway::id());

        gateway_config_pda
    }

    pub async fn init_new_mint(&mut self, mint_authority: Pubkey) -> Pubkey {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let mint_account = Keypair::new();
        let rent = self.banks_client.get_rent().await.unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &self.payer.pubkey(),
                    &mint_account.pubkey(),
                    rent.minimum_balance(Mint::LEN),
                    Mint::LEN as u64,
                    &spl_token::id(),
                ),
                spl_token::instruction::initialize_mint(
                    &spl_token::id(),
                    &mint_account.pubkey(),
                    &mint_authority,
                    None,
                    0,
                )
                .unwrap(),
            ],
            Some(&self.payer.pubkey()),
            &[&self.payer, &mint_account],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        mint_account.pubkey()
    }

    pub async fn mint_tokens_to(
        &mut self,
        mint: Pubkey,
        to: Pubkey,
        mint_authority: Keypair,
        amount: u64,
    ) {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint,
            &to,
            &mint_authority.pubkey(),
            &[&mint_authority.pubkey()],
            amount,
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, &mint_authority],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    pub async fn init_execute_data(
        &mut self,
        gateway_root_pda: &Pubkey,
        payload: Payload,
        signers: &[TestSigner],
        quorum: u128,
        nonce: u64,
        domain_separator: &[u8; 32],
    ) -> (Pubkey, Vec<u8>) {
        let (raw_data, _) = prepare_execute_data(payload, signers, quorum, nonce, domain_separator);

        let execute_data_pda = self
            .init_execute_data_with_custom_data(gateway_root_pda, &raw_data)
            .await;

        (execute_data_pda, raw_data)
    }

    pub async fn init_execute_data_with_custom_data<'a>(
        &mut self,
        gateway_root_pda: &Pubkey,
        raw_data: &'a [u8],
    ) -> Pubkey {
        let execute_data =
            GatewayExecuteData::new(raw_data, gateway_root_pda, &self.domain_separator)
                .expect("valid execute_data raw bytes");
        let (execute_data_pda, _) = execute_data.pda(gateway_root_pda);

        let (ix, _) = gateway::instructions::initialize_execute_data(
            self.payer.pubkey(),
            *gateway_root_pda,
            &self.domain_separator,
            raw_data,
        )
        .unwrap();

        self.send_tx(&[
            ComputeBudgetInstruction::set_compute_unit_limit(1399850_u32),
            ix,
        ])
        .await;

        // Confidence check: the ExecuteData account bytes contain the exact
        // execute_data raw bytes
        self.validate_recently_inscribed_execute_data(execute_data_pda, raw_data)
            .await;

        execute_data_pda
    }

    async fn validate_recently_inscribed_execute_data(
        &mut self,
        execute_data_pda: Pubkey,
        raw_data: &[u8],
    ) {
        // Confidence check: execute_data can be deserialized
        assert!(ArchivedExecuteData::from_bytes(raw_data).is_ok());

        let account = self
            .banks_client
            .get_account(execute_data_pda)
            .await
            .expect("test rpc works")
            .expect("execute_data account exists (it was just initialized)");
        assert_eq!(
            raw_data,
            account.data(),
            "inscribed execute_data bytes should match"
        );
    }

    pub async fn init_pending_gateway_commands(
        &mut self,
        gateway_root_pda: &Pubkey,
        // message and the allowed executer for the message (supposed to be a PDA owned by
        // message.destination_address)
        commands: &[OwnedCommand],
    ) -> Vec<Pubkey> {
        let ixs = commands
            .iter()
            .map(|command| {
                let (gateway_approved_message_pda, _bump, _seeds) =
                    GatewayApprovedCommand::pda(gateway_root_pda, command);
                let ix = gateway::instructions::initialize_pending_command(
                    gateway_root_pda,
                    &self.payer.pubkey(),
                    command.clone(),
                )
                .unwrap();
                (gateway_approved_message_pda, ix)
            })
            .collect::<Vec<_>>();

        let gateway_approved_command_pdas = ixs.iter().map(|(pda, _)| *pda).collect::<Vec<_>>();
        let ixs = ixs.into_iter().map(|(_, ix)| ix).collect::<Vec<_>>();
        self.send_tx(&ixs).await;

        gateway_approved_command_pdas
    }

    /// create an `execute` ix on the gateway to approve all pending PDAs
    pub async fn approve_pending_gateway_messages(
        &mut self,
        gateway_root_pda: &Pubkey,
        execute_data_pda: &Pubkey,
        approved_command_pdas: &[Pubkey],
    ) {
        let res = self
            .approve_pending_gateway_messages_with_metadata(
                gateway_root_pda,
                execute_data_pda,
                approved_command_pdas,
            )
            .await;
        assert!(res.result.is_ok());
    }

    /// create an `execute` ix on the gateway to approve all pending PDAs
    pub async fn approve_pending_gateway_messages_with_metadata(
        &mut self,
        gateway_root_pda: &Pubkey,
        execute_data_pda: &Pubkey,
        approved_command_pdas: &[Pubkey],
    ) -> BanksTransactionResultWithMetadata {
        let ix = gateway::instructions::approve_messages(
            gateway::id(),
            *execute_data_pda,
            *gateway_root_pda,
            approved_command_pdas,
        )
        .unwrap();
        let bump_budget = ComputeBudgetInstruction::set_compute_unit_limit(u32::MAX);
        self.send_tx_with_metadata(&[bump_budget, ix]).await
    }

    /// create an `RotateSigners` ix on the gateway rotate signers
    pub async fn rotate_signers_with_metadata(
        &mut self,
        gateway_root_pda: &Pubkey,
        execute_data_pda: &Pubkey,
        rotate_signers_pda: &Pubkey,
    ) -> BanksTransactionResultWithMetadata {
        let ix = gateway::instructions::rotate_signers(
            gateway::id(),
            *execute_data_pda,
            *gateway_root_pda,
            *rotate_signers_pda,
            None,
        )
        .unwrap();
        let bump_budget = ComputeBudgetInstruction::set_compute_unit_limit(650_000u32);
        self.send_tx_with_metadata(&[bump_budget, ix]).await
    }

    /// Create a new execute data PDA, command PDAs, and call
    /// gateway.approve_messages on them.
    /// Create a new execute data PDA, command PDAs, and call
    /// gateway.approve_messages on them.
    ///
    /// Returns:
    /// - approved command PDA
    /// - execute data thats stored inside the execute data PDA
    /// - execute data PDA
    pub async fn fully_approve_messages(
        &mut self,
        gateway_root_pda: &Pubkey,
        messages: Vec<Message>,
        signers: &[TestSigner],
        nonce: u64,
    ) -> (Vec<Pubkey>, Vec<u8>, Pubkey) {
        let (command_pdas, execute_data, execute_data_pda, tx) = self
            .fully_approve_messages_with_execute_metadata(
                gateway_root_pda,
                messages,
                signers,
                nonce,
            )
            .await;
        assert!(tx.result.is_ok());
        (command_pdas, execute_data, execute_data_pda)
    }

    pub async fn fully_approve_messages_with_execute_metadata(
        &mut self,
        gateway_root_pda: &Pubkey,
        messages: Vec<Message>,
        signers: &[TestSigner],
        nonce: u64,
    ) -> (
        Vec<Pubkey>,
        Vec<u8>,
        Pubkey,
        BanksTransactionResultWithMetadata,
    ) {
        let weight_of_quorum: u128 = signers
            .iter()
            .map(|signer| signer.weight)
            .try_fold(U128::ZERO, U128::checked_add)
            .and_then(|x| u128::from(x).into())
            .expect("no arithmetic overflow");

        let (execute_data_pda, execute_data) = self
            .init_execute_data(
                gateway_root_pda,
                Payload::new_messages(messages.clone()),
                signers,
                weight_of_quorum,
                nonce,
                &DOMAIN_SEPARATOR,
            )
            .await;

        let commands: Vec<_> = messages
            .into_iter()
            .map(OwnedCommand::ApproveMessage)
            .collect();

        let gateway_approved_command_pdas = self
            .init_pending_gateway_commands(gateway_root_pda, &commands)
            .await;

        let tx = self
            .approve_pending_gateway_messages_with_metadata(
                gateway_root_pda,
                &execute_data_pda,
                &gateway_approved_command_pdas,
            )
            .await;

        (
            gateway_approved_command_pdas,
            execute_data,
            execute_data_pda,
            tx,
        )
    }

    /// Create a new execute data PDA, command PDA, and call
    /// gateway.rotate_signers.
    ///
    /// Returns:
    /// - approved command PDA
    /// - execute data thats stored inside the execute data PDA
    /// - execute data PDA
    pub async fn fully_rotate_signers(
        &mut self,
        gateway_root_pda: &Pubkey,
        signer_set: VerifierSet,
        signers: &[TestSigner],
        nonce: u64,
    ) -> (Pubkey, Vec<u8>, Pubkey) {
        let (command_pdas, execute_data, execute_data_pda, tx) = self
            .fully_rotate_signers_with_execute_metadata(
                gateway_root_pda,
                signer_set,
                signers,
                nonce,
            )
            .await;
        assert!(tx.result.is_ok());
        (command_pdas, execute_data, execute_data_pda)
    }

    pub async fn fully_rotate_signers_with_execute_metadata(
        &mut self,
        gateway_root_pda: &Pubkey,
        signer_set: VerifierSet,
        signers: &[TestSigner],
        nonce: u64,
    ) -> (Pubkey, Vec<u8>, Pubkey, BanksTransactionResultWithMetadata) {
        let weight_of_quorum = signers
            .iter()
            .try_fold(U128::ZERO, |acc, i| acc.checked_add(i.weight))
            .expect("no overflow");
        let (execute_data_pda, execute_data) = self
            .init_execute_data(
                gateway_root_pda,
                Payload::VerifierSet(signer_set.clone()),
                signers,
                u128::from_le_bytes(*weight_of_quorum.to_le()),
                nonce,
                &DOMAIN_SEPARATOR,
            )
            .await;

        let command = OwnedCommand::RotateSigners(signer_set);
        let gateway_command_pda = self
            .init_pending_gateway_commands(gateway_root_pda, &[command])
            .await
            .pop()
            .unwrap();
        let tx = self
            .rotate_signers_with_metadata(gateway_root_pda, &execute_data_pda, &gateway_command_pda)
            .await;

        (gateway_command_pda, execute_data, execute_data_pda, tx)
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

    pub async fn call_execute_on_axelar_executable<'a>(
        &mut self,
        gateway_decoded_command: &OwnedCommand,
        decoded_payload: &DataPayload<'a>,
        gateway_approved_command_pda: &solana_sdk::pubkey::Pubkey,
        gateway_root_pda: solana_sdk::pubkey::Pubkey,
    ) -> solana_program_test::BanksTransactionResultWithMetadata {
        let OwnedCommand::ApproveMessage(approved_message) = gateway_decoded_command.clone() else {
            panic!("expected ApproveMessages command")
        };
        let ix = axelar_executable::construct_axelar_executable_ix(
            approved_message,
            decoded_payload.encode().unwrap(),
            *gateway_approved_command_pda,
            gateway_root_pda,
        )
        .unwrap();
        let tx = self.send_tx_with_metadata(&[ix]).await;
        assert!(tx.result.is_ok(), "transaction failed");
        tx
    }
}

pub async fn add_upgradeable_loader_account(
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

#[cfg(test)]
mod tests {

    use solana_sdk::account::Account;

    use super::*;

    /// Try to deploy the same program elf file using the
    /// `bbf_loader_upgradeable` programm directly, and regiestering the PDAs
    /// manually.
    /// The core asserts is to ensure that the account data storage
    /// is the same, thus ensuring that both operatiosn are somewhat equivelant.
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
            &programdata_account.data()[0..UpgradeableLoaderState::size_of_programdata_metadata()],
        )
        .unwrap();
        let loader_state_2 = bincode::deserialize::<UpgradeableLoaderState>(
            &programdata_account_2.data()
                [0..UpgradeableLoaderState::size_of_programdata_metadata()],
        )
        .unwrap();
        let expected_upgrade_authority_address = upgrade_authority.pubkey();
        assert!(matches!(
            loader_state,
            UpgradeableLoaderState::ProgramData {
                slot: _,
                upgrade_authority_address
            } if upgrade_authority_address == Some(expected_upgrade_authority_address)
        ));
        assert!(matches!(
            loader_state_2,
            UpgradeableLoaderState::ProgramData {
                slot: _,
                upgrade_authority_address
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
