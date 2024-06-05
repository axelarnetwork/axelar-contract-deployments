use std::ops::Add;

use account_group::instruction::GroupId;
use account_group::{get_permission_account, get_permission_group_account};
use axelar_message_primitives::command::{DecodedCommand, U256 as GatewayU256};
use axelar_message_primitives::{Address, DataPayload, EncodingScheme};
use borsh::BorshDeserialize;
use gateway::axelar_auth_weighted::AxelarAuthWeighted;
use gateway::state::{GatewayApprovedCommand, GatewayConfig, GatewayExecuteData};
use interchain_address_tracker::{get_associated_chain_address, get_associated_trusted_address};
use interchain_token_service::{
    get_flow_limiters_permission_group_id, get_interchain_token_service_root_pda,
    get_operators_permission_group_id,
};
use interchain_token_transfer_gmp::ethers_core::types::U256 as EthersU256;
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::{Bytes32, DeployTokenManager};
use itertools::{Either, Itertools};
use multisig::worker_set::WorkerSet;
use solana_program::clock::Clock;
use solana_program::hash::Hash;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_program_test::{
    BanksClient, BanksTransactionResultWithMetadata, ProgramTest, ProgramTestBanksClientExt,
    ProgramTestContext,
};
use solana_sdk::account::{AccountSharedData, WritableAccount};
use solana_sdk::account_utils::StateMut;
use solana_sdk::bpf_loader_upgradeable::{self, UpgradeableLoaderState};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::signers::Signers;
use solana_sdk::transaction::Transaction;
use spl_token::state::Mint;
use token_manager::state::TokenManagerRootAccount;
use token_manager::{get_token_manager_account, CalculatedEpoch, TokenManagerType};
pub use {connection_router, interchain_token_transfer_gmp};

use crate::account::CheckValidPDAInTests;
use crate::axelar_message::custom_message;
use crate::execute_data::{create_command_batch, sign_batch, TestSigner};

pub struct TestFixture {
    pub context: ProgramTestContext,
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub recent_blockhash: Hash,
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

    pub fn init_auth_weighted_module(&self, signers: &[TestSigner]) -> AxelarAuthWeighted {
        let total_weights = signers
            .iter()
            .map(|s| s.weight)
            .fold(GatewayU256::ZERO, |a, b| {
                let b = GatewayU256::from_le_bytes(b.to_le_bytes());
                a.checked_add(b).unwrap()
            });

        self.init_auth_weighted_module_custom_threshold(signers, total_weights)
    }

    pub fn init_auth_weighted_module_custom_threshold(
        &self,
        signers: &[TestSigner],
        threshold: GatewayU256,
    ) -> AxelarAuthWeighted {
        let addresses = signers
            .iter()
            .map(|s| {
                let address: cosmwasm_std::HexBinary = s.public_key.clone().into();
                let address = Address::try_from(address.as_slice()).unwrap();
                address
            })
            .collect_vec();
        let weights = signers
            .iter()
            .map(|s| GatewayU256::from_le_bytes(s.weight.to_le_bytes()));

        AxelarAuthWeighted::new(addresses.iter().zip(weights), threshold)
    }

    pub async fn initialize_gateway_config_account(
        &mut self,
        auth_weighted: AxelarAuthWeighted,
        init_operator: Pubkey,
    ) -> Pubkey {
        let (gateway_config_pda, bump) = GatewayConfig::pda();
        let gateway_config = GatewayConfig::new(bump, auth_weighted, init_operator);
        let ix = gateway::instructions::initialize_config(
            self.payer.pubkey(),
            gateway_config.clone(),
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
        let deserialized_gateway_config: GatewayConfig = borsh::from_slice(&account.data).unwrap();
        assert_eq!(deserialized_gateway_config, gateway_config);

        gateway_config_pda
    }

    pub async fn init_its_root_pda(
        &mut self,
        gateway_root_pda: &Pubkey,
        gas_service_root_pda: &Pubkey,
    ) -> Pubkey {
        let interchain_token_service_root_pda =
            get_interchain_token_service_root_pda(gateway_root_pda, gas_service_root_pda);
        let ix = interchain_token_service::instruction::build_initialize_instruction(
            &self.payer.pubkey(),
            &interchain_token_service_root_pda,
            gateway_root_pda,
            gas_service_root_pda,
        )
        .unwrap();
        self.send_tx(&[ix]).await;
        interchain_token_service_root_pda
    }

    pub async fn derive_token_manager_permission_groups(
        &self,
        token_id: &Bytes32,
        interchain_token_service_root_pda: &Pubkey,
        // In most cases this will be the same as `interchain_token_service_root_pda`
        init_flow_limiter: &Pubkey,
        init_operator: &Pubkey,
    ) -> ITSTokenHandlerGroups {
        let operator_group_id =
            get_operators_permission_group_id(token_id, interchain_token_service_root_pda);
        let operator_group_pda = get_permission_group_account(&operator_group_id);
        let init_operator_pda_acc = get_permission_account(&operator_group_pda, init_operator);

        let flow_group_id =
            get_flow_limiters_permission_group_id(token_id, interchain_token_service_root_pda);
        let flow_group_pda = get_permission_group_account(&flow_group_id);
        let init_flow_pda_acc = get_permission_account(&flow_group_pda, init_flow_limiter);

        ITSTokenHandlerGroups {
            operator_group: PermissionGroup {
                id: operator_group_id,
                group_pda: operator_group_pda,
                group_pda_user: init_operator_pda_acc,
                group_pda_user_owner: *init_operator,
            },
            flow_limiter_group: PermissionGroup {
                id: flow_group_id,
                group_pda: flow_group_pda,
                group_pda_user: init_flow_pda_acc,
                group_pda_user_owner: *init_flow_limiter,
            },
        }
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

    pub async fn init_new_token_manager(
        &mut self,
        interchain_token_service_root_pda: Pubkey,
        gas_service_root_pda: Pubkey,
        token_mint: Pubkey,
        gateway_root_pda: Pubkey,
        token_manager_type: TokenManagerType,
        signers: Vec<TestSigner>,
    ) -> (Pubkey, TokenManagerRootAccount, ITSTokenHandlerGroups) {
        let token_id = Bytes32(keccak256("random-token-id"));
        let init_operator = Pubkey::from([0; 32]);
        let init_flow_limiter = Pubkey::from([0; 32]);

        let its_token_manager_permission_groups = self
            .derive_token_manager_permission_groups(
                &token_id,
                &interchain_token_service_root_pda,
                &init_flow_limiter,
                &init_operator,
            )
            .await;
        let token_manager_root_pda_pubkey = get_token_manager_account(
            &its_token_manager_permission_groups.operator_group.group_pda,
            &its_token_manager_permission_groups
                .flow_limiter_group
                .group_pda,
            &interchain_token_service_root_pda,
        );
        let message_payload = interchain_token_service::instruction::from_external_chains::build_deploy_token_manager_from_gmp_instruction(
            &interchain_token_service_root_pda,
            &gas_service_root_pda,
            &self.payer.pubkey(),
            &token_manager_root_pda_pubkey,
                &its_token_manager_permission_groups.operator_group.group_pda,
                &its_token_manager_permission_groups
                    .operator_group
                    .group_pda_user_owner,
                &its_token_manager_permission_groups
                    .flow_limiter_group
                    .group_pda,
                &its_token_manager_permission_groups
                    .flow_limiter_group
                    .group_pda_user_owner,
            &token_mint,
                DeployTokenManager {
                    token_id: Bytes32(keccak256("random-token-id")),
                    token_manager_type: EthersU256::from(token_manager_type as u8),
                    params: vec![],
                },
                EncodingScheme::Borsh,
            );
        let message_to_execute =
            custom_message(interchain_token_service::id(), message_payload.clone()).unwrap();
        let (gateway_approved_message_pda, execute_data, _execute_data_pda) = self
            .fully_approve_messages(
                &gateway_root_pda,
                &[message_to_execute.clone()],
                signers.as_slice(),
            )
            .await;
        let DecodedCommand::ApproveMessages(approved_command) =
            execute_data.command_batch.commands[0].clone()
        else {
            panic!("no approved command")
        };
        let ix = axelar_executable::construct_axelar_executable_ix(
            approved_command,
            message_payload.encode().unwrap(),
            gateway_approved_message_pda[0],
            gateway_root_pda,
        )
        .unwrap();
        self.send_tx(&[ix]).await;
        let token_manager_data = self
            .banks_client
            .get_account(token_manager_root_pda_pubkey)
            .await
            .expect("get_account")
            .expect("account not none");
        let data = token_manager_data
            .check_initialized_pda::<token_manager::state::TokenManagerRootAccount>(
                &token_manager::ID,
            )
            .unwrap();
        (
            token_manager_root_pda_pubkey,
            data,
            its_token_manager_permission_groups,
        )
    }

    /// Returns token manager root pda
    pub async fn setup_token_manager(
        &mut self,
        token_manager_type: TokenManagerType,
        groups: ITSTokenHandlerGroups,
        flow_limit: u64,
        gateway_root_config_pda: Pubkey,
        token_mint: Pubkey,
        its_pda: Pubkey,
    ) -> Pubkey {
        let token_manager_pda = token_manager::get_token_manager_account(
            &groups.operator_group.group_pda,
            &groups.flow_limiter_group.group_pda,
            &its_pda,
        );
        let clock = self.banks_client.get_sysvar::<Clock>().await.unwrap();
        let block_timestamp = clock.unix_timestamp;

        let _token_flow_pda = token_manager::get_token_flow_account(
            &token_manager_pda,
            CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
        );

        let ix = token_manager::instruction::build_setup_instruction(
            &self.payer.pubkey(),
            &token_manager_pda,
            &groups.operator_group.group_pda,
            &groups.operator_group.group_pda_user_owner,
            &groups.flow_limiter_group.group_pda,
            &groups.flow_limiter_group.group_pda_user_owner,
            &its_pda,
            &token_mint,
            &gateway_root_config_pda,
            token_manager::instruction::Setup {
                flow_limit,
                token_manager_type,
            },
        )
        .unwrap();
        self.send_tx(&[ix]).await;
        token_manager_pda
    }

    pub async fn setup_permission_group(&mut self, group: &PermissionGroup) {
        let ix = account_group::instruction::build_setup_permission_group_instruction(
            &self.payer.pubkey(),
            &group.group_pda,
            &group.group_pda_user,
            &group.group_pda_user_owner,
            group.id.clone(),
        )
        .unwrap();
        self.send_tx(&[ix]).await;
    }

    pub async fn init_execute_data(
        &mut self,
        gateway_root_pda: &Pubkey,
        messages: &[Either<connection_router::Message, WorkerSet>],
        signers: &[TestSigner],
        quorum: u128,
    ) -> (Pubkey, GatewayExecuteData, Vec<u8>) {
        let (execute_data, raw_data) =
            prepare_execute_data(messages, signers, quorum, gateway_root_pda);

        let execute_data_pda = self
            .init_execute_data_with_custom_data(gateway_root_pda, &raw_data, &execute_data)
            .await;

        (execute_data_pda, execute_data, raw_data)
    }

    pub async fn init_execute_data_with_custom_data(
        &mut self,
        gateway_root_pda: &Pubkey,
        raw_data: &[u8],
        execute_data: &GatewayExecuteData,
    ) -> Pubkey {
        let (execute_data_pda, _, _) = execute_data.pda(gateway_root_pda);

        let (ix, _) = gateway::instructions::initialize_execute_data(
            self.payer.pubkey(),
            *gateway_root_pda,
            raw_data.to_vec(),
        )
        .unwrap();

        self.send_tx(&[
            ComputeBudgetInstruction::set_compute_unit_limit(1399850_u32),
            ix,
        ])
        .await;

        execute_data_pda
    }

    pub async fn init_pending_gateway_commands(
        &mut self,
        gateway_root_pda: &Pubkey,
        // message and the allowed executer for the message (supposed to be a PDA owned by
        // message.destination_address)
        commands: &[DecodedCommand],
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
        let bump_budget = ComputeBudgetInstruction::set_compute_unit_limit(400_000u32);
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
        let bump_budget = ComputeBudgetInstruction::set_compute_unit_limit(400_000u32);
        self.send_tx_with_metadata(&[bump_budget, ix]).await
    }

    pub async fn prepare_trusted_address_iatracker(
        &mut self,
        owner: Keypair,
        trusted_chain_name: String,
        trusted_chain_addr: String,
    ) -> (Pubkey, String) {
        let associated_chain_address = get_associated_chain_address(&owner.pubkey());

        let recent_blockhash = self.refresh_blockhash().await;
        let ix =
            interchain_address_tracker::instruction::build_create_registered_chain_instruction(
                &self.payer.pubkey(),
                &associated_chain_address,
                &owner.pubkey(),
                trusted_chain_name.clone(),
            )
            .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, &owner],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        let associated_trusted_address =
            get_associated_trusted_address(&associated_chain_address, &trusted_chain_name);

        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = interchain_address_tracker::instruction::build_set_trusted_address_instruction(
            &self.payer.pubkey(),
            &associated_chain_address,
            &associated_trusted_address,
            &owner.pubkey(),
            trusted_chain_name,
            trusted_chain_addr.clone(),
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, &owner],
            recent_blockhash,
        );

        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        // Associated account now exists
        let associated_account = self
            .banks_client
            .get_account(associated_trusted_address)
            .await
            .expect("get_account")
            .expect("associated_account not none");
        let account_info =
            interchain_address_tracker::state::RegisteredTrustedAddressAccount::unpack_from_slice(
                associated_account.data.as_slice(),
            )
            .unwrap();
        assert_eq!(account_info.address, trusted_chain_addr);
        associated_account.check_initialized_pda::<interchain_address_tracker::state::RegisteredTrustedAddressAccount>(
            &interchain_address_tracker::id(),
        )
        .unwrap();

        (associated_trusted_address, account_info.address)
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
        messages: &[connection_router::Message],
        signers: &[TestSigner],
    ) -> (Vec<Pubkey>, GatewayExecuteData, Pubkey) {
        let (command_pdas, execute_data, execute_data_pda, tx) = self
            .fully_approve_messages_with_execute_metadata(gateway_root_pda, messages, signers)
            .await;
        assert!(tx.result.is_ok());
        (command_pdas, execute_data, execute_data_pda)
    }

    pub async fn fully_approve_messages_with_execute_metadata(
        &mut self,
        gateway_root_pda: &Pubkey,
        messages: &[connection_router::Message],
        signers: &[TestSigner],
    ) -> (
        Vec<Pubkey>,
        GatewayExecuteData,
        Pubkey,
        BanksTransactionResultWithMetadata,
    ) {
        let weight_of_quorum = signers
            .iter()
            .fold(cosmwasm_std::Uint256::zero(), |acc, i| acc.add(i.weight));
        let weight_of_quorum = EthersU256::from_big_endian(&weight_of_quorum.to_be_bytes());
        let (execute_data_pda, execute_data, _) = self
            .init_execute_data(
                gateway_root_pda,
                &messages
                    .iter()
                    .map(|x| Either::Left(x.clone()))
                    .collect_vec(),
                signers,
                weight_of_quorum.as_u128(),
            )
            .await;
        let gateway_approved_command_pdas = self
            .init_pending_gateway_commands(
                gateway_root_pda,
                execute_data.command_batch.commands.as_ref(),
            )
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
        signer_set: WorkerSet,
        signers: &[TestSigner],
    ) -> (Pubkey, GatewayExecuteData, Pubkey) {
        let (command_pdas, execute_data, execute_data_pda, tx) = self
            .fully_rotate_signers_with_execute_metadata(gateway_root_pda, signer_set, signers)
            .await;
        assert!(tx.result.is_ok());
        (command_pdas, execute_data, execute_data_pda)
    }

    pub async fn fully_rotate_signers_with_execute_metadata(
        &mut self,
        gateway_root_pda: &Pubkey,
        signer_set: WorkerSet,
        signers: &[TestSigner],
    ) -> (
        Pubkey,
        GatewayExecuteData,
        Pubkey,
        BanksTransactionResultWithMetadata,
    ) {
        let weight_of_quorum = signers
            .iter()
            .fold(cosmwasm_std::Uint256::zero(), |acc, i| acc.add(i.weight));
        let weight_of_quorum = EthersU256::from_big_endian(&weight_of_quorum.to_be_bytes());
        let (execute_data_pda, execute_data, _) = self
            .init_execute_data(
                gateway_root_pda,
                &[Either::Right(signer_set)],
                signers,
                weight_of_quorum.as_u128(),
            )
            .await;
        let gateway_command_pda = self
            .init_pending_gateway_commands(
                gateway_root_pda,
                execute_data.command_batch.commands.as_ref(),
            )
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
        gateway_decoded_command: &DecodedCommand,
        decoded_payload: &DataPayload<'a>,
        gateway_approved_command_pda: &solana_sdk::pubkey::Pubkey,
        gateway_root_pda: solana_sdk::pubkey::Pubkey,
    ) -> solana_program_test::BanksTransactionResultWithMetadata {
        let DecodedCommand::ApproveMessages(approved_message) = gateway_decoded_command.clone()
        else {
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

pub fn prepare_execute_data(
    messages: &[Either<connection_router::Message, WorkerSet>],
    signers: &[TestSigner],
    quorum: u128,
    gateway_root_pda: &Pubkey,
) -> (GatewayExecuteData, Vec<u8>) {
    let command_batch = create_command_batch(messages).unwrap();
    let signatures = sign_batch(&command_batch, signers).unwrap();
    let encoded_message =
        crate::execute_data::encode(&command_batch, signers.to_vec(), signatures, quorum).unwrap();
    let execute_data = GatewayExecuteData::new(encoded_message.as_ref(), gateway_root_pda).unwrap();
    (execute_data, encoded_message)
}

#[derive(Debug, Clone)]
pub struct PermissionGroup {
    pub id: GroupId,
    pub group_pda: Pubkey,
    pub group_pda_user: Pubkey,
    pub group_pda_user_owner: Pubkey,
}

#[derive(Debug, Clone)]
pub struct ITSTokenHandlerGroups {
    pub operator_group: PermissionGroup,
    pub flow_limiter_group: PermissionGroup,
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

    use solana_sdk::account::{Account, ReadableAccount};

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
                programdata_address: programdata_pda
            }
        ));
        assert!(matches!(
            loader_state_2,
            UpgradeableLoaderState::Program {
                programdata_address: programdata_pda_2
            }
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
        let upgrade_authhority_address = upgrade_authority.pubkey();
        assert!(matches!(
            loader_state,
            UpgradeableLoaderState::ProgramData {
                slot: _,
                upgrade_authority_address
            }
        ));
        assert!(matches!(
            loader_state_2,
            UpgradeableLoaderState::ProgramData {
                slot: _,
                upgrade_authority_address
            }
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
