use std::path::PathBuf;

use axelar_message_primitives::{DataPayload, U256};
use axelar_rkyv_encoding::rkyv::de::deserializers::SharedDeserializeMap;
use axelar_rkyv_encoding::rkyv::validation::validators::DefaultValidator;
use axelar_rkyv_encoding::rkyv::{Archive, CheckBytes, Deserialize};
use axelar_rkyv_encoding::types::u128::U128;
use axelar_rkyv_encoding::types::{ExecuteData, Message, Payload, VerifierSet};
use borsh::BorshDeserialize;
use gateway::commands::OwnedCommand;
use gateway::hasher_impl;
use gateway::instructions::{InitializeConfig, VerifierSetWrapper};
use gateway::processor::ToBytes;
use gateway::state::execute_data::{
    ApproveMessagesVariant, ArchivedGatewayExecuteData, ExecuteDataVariant, RotateSignersVariant,
};
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
use spl_token_2022::extension::ExtensionType;
use spl_token_2022::state::Mint;
pub use {connection_router, interchain_token_transfer_gmp};

use crate::account::CheckValidPDAInTests;
use crate::execute_data::prepare_execute_data;
use crate::test_signer::{create_signer_with_weight, TestSigner};

pub struct TestFixture {
    pub context: ProgramTestContext,
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub recent_blockhash: Hash,
}

#[derive(Clone, Debug)]
pub struct SigningVerifierSet {
    pub signers: Vec<TestSigner>,
    pub nonce: u64,
    pub quorum: U128,
    pub domain_separator: [u8; 32],
}

impl SigningVerifierSet {
    pub fn new(signers: Vec<TestSigner>, nonce: u64, domain_separator: [u8; 32]) -> Self {
        let quorum = signers
            .iter()
            .map(|signer| signer.weight)
            .try_fold(U128::ZERO, U128::checked_add)
            .expect("no arithmetic overflow");
        Self::new_with_quorum(signers, nonce, quorum, domain_separator)
    }

    pub fn new_with_quorum(
        signers: Vec<TestSigner>,
        nonce: u64,
        quorum: U128,
        domain_separator: [u8; 32],
    ) -> Self {
        Self {
            signers,
            nonce,
            quorum,
            domain_separator,
        }
    }

    pub fn verifier_set_tracker(&self) -> Pubkey {
        gateway::get_verifier_set_tracker_pda(
            &gateway::id(),
            self.verifier_set().hash(hasher_impl()),
        )
        .0
    }

    pub fn verifier_set(&self) -> VerifierSet {
        let signers = self
            .signers
            .iter()
            .map(|x| (x.public_key, x.weight))
            .collect();
        VerifierSet::new(self.nonce, signers, self.quorum, self.domain_separator)
    }
}

impl TestFixture {
    pub async fn new(pt: ProgramTest) -> TestFixture {
        let context = pt.start_with_context().await;
        TestFixture {
            banks_client: context.banks_client.clone(),
            payer: context.payer.insecure_clone(),
            recent_blockhash: context.last_blockhash,
            context,
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

    pub async fn set_time(&mut self, time: i64) {
        // get clock sysvar
        let clock_sysvar: Clock = self.banks_client.get_sysvar().await.unwrap();

        // update clock
        let mut new_clock = clock_sysvar.clone();
        new_clock.unix_timestamp = time;

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
            |acc| acc.set_executable(true),
        )
        .await;
        let programdata_data_offset = UpgradeableLoaderState::size_of_programdata_metadata();
        let program_data_len = program_bytecode.len() + programdata_data_offset;

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
                    .copy_from_slice(program_bytecode);
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

    pub fn create_verifier_sets(&self, signers: &[&SigningVerifierSet]) -> Vec<VerifierSetWrapper> {
        signers
            .iter()
            .map(|set| VerifierSetWrapper::new_from_verifier_set(set.verifier_set()).unwrap())
            .collect_vec()
    }

    pub fn base_initialize_config(
        &self,
        domain_separator: [u8; 32],
    ) -> InitializeConfig<VerifierSetWrapper> {
        InitializeConfig {
            domain_separator,
            initial_signer_sets: vec![],
            minimum_rotation_delay: 0,
            operator: Pubkey::new_unique(),
            previous_signers_retention: U256::from(0_u128),
        }
    }

    pub async fn initialize_gateway_config_account(
        &mut self,
        init_config: InitializeConfig<VerifierSetWrapper>,
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

    pub async fn init_new_mint(
        &mut self,
        mint_authority: Pubkey,
        token_program_id: Pubkey,
        decimals: u8,
    ) -> Pubkey {
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
                    &token_program_id,
                ),
                spl_token_2022::instruction::initialize_mint(
                    &token_program_id,
                    &mint_account.pubkey(),
                    &mint_authority,
                    None,
                    decimals,
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

    #[allow(clippy::too_many_arguments)]
    pub async fn init_new_mint_with_fee(
        &mut self,
        mint_authority: Pubkey,
        token_program_id: Pubkey,
        fee_basis_points: u16,
        maximum_fee: u64,
        decimals: u8,
        transfer_fee_config_authority: Option<&Pubkey>,
        withdraw_withheld_authority: Option<&Pubkey>,
    ) -> Pubkey {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let mint_account = Keypair::new();
        let rent = self.banks_client.get_rent().await.unwrap();
        let space =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
                .unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &self.payer.pubkey(),
                    &mint_account.pubkey(),
                    rent.minimum_balance(space),
                    space as u64,
                    &token_program_id,
                ),
                spl_token_2022::extension::transfer_fee::instruction::initialize_transfer_fee_config(
                    &token_program_id,
                    &mint_account.pubkey(),
                    transfer_fee_config_authority,
                    withdraw_withheld_authority,
                    fee_basis_points,
                    maximum_fee
                ).unwrap(),
                spl_token_2022::instruction::initialize_mint(
                    &token_program_id,
                    &mint_account.pubkey(),
                    &mint_authority,
                    None,
                    decimals
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
        token_program_id: Pubkey,
    ) {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = spl_token_2022::instruction::mint_to(
            &token_program_id,
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

    pub async fn init_approve_messages_execute_data(
        &mut self,
        gateway_root_pda: &Pubkey,
        payload: Payload,
        signers: &SigningVerifierSet,
        domain_separator: &[u8; 32],
    ) -> (
        Pubkey,
        ExecuteData,
        GatewayExecuteData<ApproveMessagesVariant>,
    ) {
        let (raw_data, _) = prepare_execute_data(payload, signers, domain_separator);
        let execute_data_pda = self
            .init_approve_messages_execute_data_with_custom_data(
                gateway_root_pda,
                &raw_data.to_bytes::<0>().unwrap(),
                domain_separator,
            )
            .await;
        let execute_data = GatewayExecuteData::new(
            &raw_data.to_bytes::<0>().unwrap(),
            gateway_root_pda,
            domain_separator,
        )
        .unwrap();

        (execute_data_pda, raw_data, execute_data)
    }

    pub async fn init_rotate_signers_execute_data(
        &mut self,
        gateway_root_pda: &Pubkey,
        payload: Payload,
        signers: &SigningVerifierSet,
        domain_separator: &[u8; 32],
    ) -> (
        Pubkey,
        ExecuteData,
        GatewayExecuteData<RotateSignersVariant>,
    ) {
        let (raw_data, _) = prepare_execute_data(payload, signers, domain_separator);
        let execute_data_pda = self
            .init_rotate_signers_execute_data_with_custom_data(
                gateway_root_pda,
                &raw_data.to_bytes::<0>().unwrap(),
                domain_separator,
            )
            .await;
        let execute_data = GatewayExecuteData::new(
            &raw_data.to_bytes::<0>().unwrap(),
            gateway_root_pda,
            domain_separator,
        )
        .unwrap();

        (execute_data_pda, raw_data, execute_data)
    }

    pub async fn init_execute_data(
        &mut self,
        gateway_root_pda: &Pubkey,
        payload: Payload,
        signers: &SigningVerifierSet,
        domain_separator: &[u8; 32],
    ) -> (Pubkey, Vec<u8>) {
        match &payload {
            Payload::Messages(_) => {
                let res = self
                    .init_approve_messages_execute_data(
                        gateway_root_pda,
                        payload,
                        signers,
                        domain_separator,
                    )
                    .await;
                (res.0, res.1.to_bytes::<0>().unwrap())
            }
            Payload::VerifierSet(_) => {
                let res = self
                    .init_rotate_signers_execute_data(
                        gateway_root_pda,
                        payload,
                        signers,
                        domain_separator,
                    )
                    .await;
                (res.0, res.1.to_bytes::<0>().unwrap())
            }
        }
    }

    pub async fn init_approve_messages_execute_data_with_custom_data<'a>(
        &mut self,
        gateway_root_pda: &Pubkey,
        raw_data: &[u8],
        domain_separator: &[u8; 32],
    ) -> Pubkey {
        let execute_data = GatewayExecuteData::<ApproveMessagesVariant>::new(
            raw_data,
            gateway_root_pda,
            domain_separator,
        )
        .expect("valid execute_data raw bytes");
        let (execute_data_pda, _) =
            gateway::get_execute_data_pda(gateway_root_pda, &execute_data.hash_decoded_contents());

        let (ix, _) = gateway::instructions::initialize_approve_messages_execute_data(
            self.payer.pubkey(),
            *gateway_root_pda,
            domain_separator,
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
        let bytes = execute_data.to_bytes().unwrap();
        self.validate_recently_inscribed_execute_data::<ApproveMessagesVariant>(
            execute_data_pda,
            &bytes,
        )
        .await;

        execute_data_pda
    }

    pub async fn init_rotate_signers_execute_data_with_custom_data(
        &mut self,
        gateway_root_pda: &Pubkey,
        raw_data: &[u8],
        domain_separator: &[u8; 32],
    ) -> Pubkey {
        let execute_data = GatewayExecuteData::<RotateSignersVariant>::new(
            raw_data,
            gateway_root_pda,
            domain_separator,
        )
        .expect("valid execute_data raw bytes");
        let (execute_data_pda, _) =
            gateway::get_execute_data_pda(gateway_root_pda, &execute_data.hash_decoded_contents());

        let (ix, _) = gateway::instructions::initialize_rotate_signers_execute_data(
            self.payer.pubkey(),
            *gateway_root_pda,
            domain_separator,
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
        let bytes = execute_data.to_bytes().unwrap();
        self.validate_recently_inscribed_execute_data::<RotateSignersVariant>(
            execute_data_pda,
            &bytes,
        )
        .await;

        execute_data_pda
    }

    async fn validate_recently_inscribed_execute_data<'a, T>(
        &mut self,
        execute_data_pda: Pubkey,
        raw_data: &'a [u8],
    ) where
        T: ExecuteDataVariant + 'a,
        T::ArchivedData: CheckBytes<DefaultValidator<'a>>,
    {
        // Confidence check: execute_data can be deserialized
        assert!(ArchivedGatewayExecuteData::<T>::from_bytes(raw_data).is_ok());

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
        verifier_set_tracker: &Pubkey,
    ) {
        let res = self
            .approve_pending_gateway_messages_with_metadata(
                gateway_root_pda,
                execute_data_pda,
                approved_command_pdas,
                verifier_set_tracker,
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
        verifier_set_tracker: &Pubkey,
    ) -> BanksTransactionResultWithMetadata {
        let ix = gateway::instructions::approve_messages(
            *execute_data_pda,
            *gateway_root_pda,
            approved_command_pdas,
            *verifier_set_tracker,
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
        current_verifier_set_tracker_pda: &Pubkey,
        new_verifier_set_tracker_pda: &Pubkey,
    ) -> BanksTransactionResultWithMetadata {
        let ix = gateway::instructions::rotate_signers(
            *execute_data_pda,
            *gateway_root_pda,
            None,
            *current_verifier_set_tracker_pda,
            *new_verifier_set_tracker_pda,
            self.payer.pubkey(),
        )
        .unwrap();
        let bump_budget = ComputeBudgetInstruction::set_compute_unit_limit(u32::MAX);
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
        signers: &SigningVerifierSet,
        domain_separator: &[u8; 32],
    ) -> (Vec<Pubkey>, Vec<u8>, Pubkey) {
        let (command_pdas, execute_data, execute_data_pda, tx) = self
            .fully_approve_messages_with_execute_metadata(
                gateway_root_pda,
                messages,
                signers,
                domain_separator,
            )
            .await;
        assert!(tx.result.is_ok());
        (command_pdas, execute_data, execute_data_pda)
    }

    pub async fn fully_approve_messages_with_execute_metadata(
        &mut self,
        gateway_root_pda: &Pubkey,
        messages: Vec<Message>,
        signers: &SigningVerifierSet,
        domain_separator: &[u8; 32],
    ) -> (
        Vec<Pubkey>,
        Vec<u8>,
        Pubkey,
        BanksTransactionResultWithMetadata,
    ) {
        let verifier_set_tracker = signers.verifier_set_tracker();
        let (execute_data_pda, execute_data) = self
            .init_execute_data(
                gateway_root_pda,
                Payload::new_messages(messages.clone()),
                signers,
                domain_separator,
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
                &verifier_set_tracker,
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
    /// - execute data thats stored inside the execute data PDA
    /// - execute data PDA
    pub async fn fully_rotate_signers(
        &mut self,
        gateway_root_pda: &Pubkey,
        new_signer_set: VerifierSet,
        signers: &SigningVerifierSet,
        domain_separator: &[u8; 32],
    ) -> (Vec<u8>, Pubkey) {
        let (execute_data, execute_data_pda, tx) = self
            .fully_rotate_signers_with_execute_metadata(
                gateway_root_pda,
                new_signer_set,
                signers,
                domain_separator,
            )
            .await;
        assert!(tx.result.is_ok());
        (execute_data, execute_data_pda)
    }

    pub async fn fully_rotate_signers_with_execute_metadata(
        &mut self,
        gateway_root_pda: &Pubkey,
        new_signer_set: VerifierSet,
        signers: &SigningVerifierSet,
        domain_separator: &[u8; 32],
    ) -> (Vec<u8>, Pubkey, BanksTransactionResultWithMetadata) {
        let current_verifier_set_tracker_pda = signers.verifier_set_tracker();
        let (new_verifier_set_tracker_pda, _) =
            gateway::get_verifier_set_tracker_pda(&gateway::ID, new_signer_set.hash(hasher_impl()));
        let (execute_data_pda, execute_data) = self
            .init_execute_data(
                gateway_root_pda,
                Payload::VerifierSet(new_signer_set.clone()),
                signers,
                domain_separator,
            )
            .await;
        let tx = self
            .rotate_signers_with_metadata(
                gateway_root_pda,
                &execute_data_pda,
                &current_verifier_set_tracker_pda,
                &new_verifier_set_tracker_pda,
            )
            .await;

        (execute_data, execute_data_pda, tx)
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

    pub async fn get_account_raw_bytes(
        &mut self,
        account: &Pubkey,
        expected_owner: &Pubkey,
    ) -> Vec<u8> {
        let account = self
            .banks_client
            .get_account(*account)
            .await
            .expect("get_account")
            .expect("account not none");
        let data = account
            .check_initialized_pda_raw_bytes(expected_owner)
            .unwrap();
        data.to_owned()
    }

    pub async fn get_rkyv_account<T>(&mut self, account: &Pubkey, expected_owner: &Pubkey) -> T
    where
        T: Archive,
        T::Archived: Deserialize<T, SharedDeserializeMap>,
    {
        let account = self
            .banks_client
            .get_account(*account)
            .await
            .expect("get_account")
            .expect("account not none");
        account
            .check_rkyv_initialized_pda::<T>(expected_owner)
            .unwrap()
    }

    pub async fn call_execute_on_axelar_executable<'a>(
        &mut self,
        gateway_decoded_command: &OwnedCommand,
        decoded_payload: &DataPayload<'a>,
        gateway_approved_command_pda: &solana_sdk::pubkey::Pubkey,
        gateway_root_pda: &solana_sdk::pubkey::Pubkey,
    ) -> solana_program_test::BanksTransactionResultWithMetadata {
        let OwnedCommand::ApproveMessage(approved_message) = gateway_decoded_command.clone() else {
            panic!("expected ApproveMessages command")
        };
        let ix = axelar_executable_old::construct_axelar_executable_ix(
            approved_message,
            decoded_payload.encode().unwrap(),
            *gateway_approved_command_pda,
            *gateway_root_pda,
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

/// Contains metadata information about the initialised Gateway config
pub struct SolanaAxelarIntegrationMetadata {
    pub fixture: TestFixture,
    pub signers: SigningVerifierSet,
    pub gateway_root_pda: Pubkey,
    pub operator: Keypair,
    pub upgrade_authority: Keypair,
    pub domain_separator: [u8; 32],
}

#[derive(Debug, typed_builder::TypedBuilder)]
pub struct SolanaAxelarIntegration {
    #[builder(default)]
    initial_signer_weights: Vec<u128>,
    #[builder(default, setter(strip_option))]
    custom_quorum: Option<u128>,
    #[builder(default)]
    minimum_rotate_signers_delay_seconds: u64,
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
    const NONCE: u64 = 42;
    const DOMAIN_SEPARATOR: [u8; 32] = [42; 32];

    pub async fn setup(self) -> SolanaAxelarIntegrationMetadata {
        // Create a new ProgramTest instance
        let fixture = TestFixture::new(ProgramTest::default()).await;
        self.setup_with_fixture(fixture).await
    }

    pub async fn setup_with_fixture(
        self,
        mut fixture: TestFixture,
    ) -> SolanaAxelarIntegrationMetadata {
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
        let gateway_program_bytecode = tokio::fs::read("../../target/deploy/gmp_gateway.so")
            .await
            .unwrap();
        fixture
            .register_upgradeable_program(
                &gateway_program_bytecode,
                &upgrade_authority.pubkey(),
                &gateway::id(),
            )
            .await;

        // initialize the gateway
        let initial_signers = make_signers_with_quorum(
            &self.initial_signer_weights,
            Self::NONCE,
            self.custom_quorum
                .unwrap_or_else(|| self.initial_signer_weights.iter().sum()),
            Self::DOMAIN_SEPARATOR,
        );
        let operator = Keypair::new();
        let gateway_root_pda = fixture
            .initialize_gateway_config_account(InitializeConfig {
                initial_signer_sets: fixture.create_verifier_sets(&[&initial_signers]),
                operator: operator.pubkey(),
                previous_signers_retention: U256::from_u64(self.previous_signers_retention),
                minimum_rotation_delay: self.minimum_rotate_signers_delay_seconds,
                ..fixture.base_initialize_config(Self::DOMAIN_SEPARATOR)
            })
            .await;

        SolanaAxelarIntegrationMetadata {
            domain_separator: Self::DOMAIN_SEPARATOR,
            upgrade_authority,
            fixture,
            signers: initial_signers,
            gateway_root_pda,
            operator,
        }
    }
}

pub fn make_signers(
    weights: &[u128],
    nonce: u64,
    domain_separator: [u8; 32],
) -> SigningVerifierSet {
    let signers = weights
        .iter()
        .copied()
        .map(create_signer_with_weight)
        .collect::<Vec<_>>();

    SigningVerifierSet::new(signers, nonce, domain_separator)
}

pub fn make_signers_with_quorum(
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

    SigningVerifierSet::new_with_quorum(signers, nonce, U128::from(quorum), domain_separator)
}

pub fn workspace_root_dir() -> PathBuf {
    let dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    PathBuf::from(dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
