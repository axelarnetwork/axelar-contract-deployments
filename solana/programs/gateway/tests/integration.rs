// #![cfg(feature = "test-sbf")]

mod common;

use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use common::program_test;
use gateway::accounts::{GatewayApprovedMessage, GatewayConfig, GatewayExecuteData};
use gateway::events::GatewayEvent;
use gateway::find_root_pda;
use random_array::rand_array;
use solana_program::address_lookup_table::state::AddressLookupTable;
use solana_program::address_lookup_table_account::AddressLookupTableAccount;
use solana_program::hash::Hash;
use solana_program::instruction::Instruction;
use solana_program::message::v0::Message;
use solana_program::message::VersionedMessage;
use solana_program::pubkey::Pubkey;
use solana_program::slot_hashes::SlotHashes;
use solana_program::slot_history::Slot;
use solana_program_test::{
    tokio, BanksClient, BanksTransactionResultWithMetadata, ProgramTestContext,
};
use solana_sdk::address_lookup_table::instruction::{create_lookup_table, extend_lookup_table};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, VersionedTransaction};

mod helper {
    use super::*;
    pub async fn prepare_versioned_transaction(
        context: &mut ProgramTestContext,
        instruction: Instruction,
        lookup_table_address: Pubkey,
    ) -> Result<VersionedTransaction> {
        let raw_account = context
            .banks_client
            .get_account(lookup_table_address)
            .await?
            .ok_or(anyhow!("could not find address lookup table account"))?;
        let address_lookup_table = AddressLookupTable::deserialize(&raw_account.data)?;
        let address_lookup_table_account = AddressLookupTableAccount {
            key: lookup_table_address,
            addresses: address_lookup_table.addresses.to_vec(),
        };
        let blockhash = context.banks_client.get_latest_blockhash().await?;
        let message = Message::try_compile(
            &context.payer.pubkey(),
            &[instruction],
            &[address_lookup_table_account],
            blockhash,
        )?;
        Ok(VersionedTransaction::try_new(
            VersionedMessage::V0(message),
            &[&context.payer],
        )?)
    }
}

mod accounts {
    use super::*;
    pub(super) async fn initialize_config_account(
        client: &mut BanksClient,
        payer: &Keypair,
        gateway_config: &GatewayConfig,
    ) -> Result<()> {
        let recent_blockhash = client.get_latest_blockhash().await?;
        let (gateway_config_pda, _bump) = GatewayConfig::pda();

        let ix = gateway::instructions::initialize_config(payer.pubkey(), gateway_config.clone())?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );

        client.process_transaction(tx).await?;

        let account = client
            .get_account(gateway_config_pda)
            .await?
            .expect("metadata");

        assert_eq!(account.owner, gateway::id());
        let deserialized_gateway_config: GatewayConfig = borsh::from_slice(&account.data)?;
        assert_eq!(deserialized_gateway_config, *gateway_config);

        Ok(())
    }

    pub(super) async fn initialize_execute_data_account(
        context: &mut ProgramTestContext,
        pda: Pubkey,
        execute_data: GatewayExecuteData,
    ) -> Result<()> {
        let recent_blockhash = context.banks_client.get_latest_blockhash().await?;
        let ix = gateway::instructions::initialize_execute_data(
            context.payer.pubkey(),
            pda,
            execute_data.clone(),
        )?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            recent_blockhash,
        );

        context.banks_client.process_transaction(tx).await?;

        let account = context
            .banks_client
            .get_account(pda)
            .await?
            .expect("metadata");

        assert_eq!(account.owner, gateway::id());
        let deserialized_execute_data: GatewayExecuteData = borsh::from_slice(&account.data)?;
        assert_eq!(deserialized_execute_data, execute_data);

        Ok(())
    }

    pub(super) async fn initialize_message(
        client: &mut BanksClient,
        payer: Keypair,
        pda: Pubkey,
        approved_message: GatewayApprovedMessage,
    ) -> Result<()> {
        let recent_blockhash = client.get_latest_blockhash().await?;
        let ix = gateway::instructions::initialize_messge(
            payer.pubkey(),
            pda,
            approved_message.clone(),
        )?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        client.process_transaction(tx).await?;

        let account = client.get_account(pda).await?.expect("metadata");
        assert_eq!(account.owner, gateway::id());
        let deserialized_execute_data: GatewayApprovedMessage = borsh::from_slice(&account.data)?;
        assert_eq!(deserialized_execute_data, approved_message);

        Ok(())
    }

    /// Creates an Address Lookup Table.
    /// Payer is also assigned as the Authority of the lookup table.
    pub(super) async fn create_address_lookup_table(
        context: &mut ProgramTestContext,
        payer: Pubkey,
        slot: u64,
    ) -> Result<Pubkey> {
        let (create_ix, lookup_table_address) = create_lookup_table(payer, payer, slot);
        let recent_block = context.banks_client.get_latest_blockhash().await?;
        let create_tx = Transaction::new_signed_with_payer(
            &[create_ix],
            Some(&payer),
            &[&context.payer],
            recent_block,
        );
        context.banks_client.process_transaction(create_tx).await?;
        Ok(lookup_table_address)
    }

    /// Extends an Address Lookup Table with the provided accounts.
    /// Payer is also assigned as the Authority of the lookup table.
    pub(super) async fn extend_address_lookup_table(
        context: &mut ProgramTestContext,
        lookup_table_address: Pubkey,
        payer: Pubkey,
        accounts: &[Pubkey],
    ) -> Result<()> {
        let extend_ix =
            extend_lookup_table(lookup_table_address, payer, Some(payer), accounts.to_vec());
        let recent_block = context.banks_client.get_latest_blockhash().await?;
        let extend_tx = Transaction::new_signed_with_payer(
            &[extend_ix],
            Some(&payer),
            &[&context.payer],
            recent_block,
        );
        context.banks_client.process_transaction(extend_tx).await?;
        Ok(())
    }

    /// Helper struct to track and advance slots in tests.
    pub struct SlotTracker {
        position: usize,
        slots: Vec<Slot>,
    }

    impl<'a> SlotTracker {
        pub fn new(context: &ProgramTestContext, slots: Vec<Slot>) -> Result<Self> {
            anyhow::ensure!(!slots.is_empty());
            Self::overwrite_slot_hashes_with_slots(context, &slots);
            Ok(Self { position: 0, slots })
        }

        fn overwrite_slot_hashes_with_slots(context: &ProgramTestContext, slots: &[Slot]) {
            let mut slot_hashes = SlotHashes::default();
            for slot in slots {
                slot_hashes.add(*slot, Hash::new_unique());
            }
            context.set_sysvar(&slot_hashes);
        }

        pub fn warp_to_next_slot(&mut self, context: &'a mut ProgramTestContext) -> Result<()> {
            let new_position = self.position + 1;
            anyhow::ensure!(new_position < self.slots.len(), "No more slots to advance");
            self.position = new_position;
            context.warp_to_slot(self.current_slot())?;
            Ok(())
        }

        pub fn current_slot(&self) -> Slot {
            self.slots[self.position]
        }
    }

    /// Original function refactored to use the new functions.
    /// Payer is also assigned as the Authority of the lookup table.
    pub(super) async fn initialize_address_lookup_table(
        context: &mut ProgramTestContext,
        accounts: &[Pubkey],
        slot_tracker: &mut SlotTracker,
    ) -> Result<Pubkey> {
        assert!(accounts.len() < u8::MAX as usize, "too many accounts");
        let payer_pk = context.payer.pubkey();
        let lookup_table_address =
            create_address_lookup_table(context, payer_pk, slot_tracker.current_slot()).await?;
        // Need to wait for the next slot to extended the created address lookup table.
        slot_tracker.warp_to_next_slot(context)?;
        extend_address_lookup_table(context, lookup_table_address, payer_pk, accounts).await?;
        // Need to wait for the next slot to use the extended address lookup table.
        slot_tracker.warp_to_next_slot(context)?;
        Ok(lookup_table_address)
    }
}

#[tokio::test]
async fn test_call_contract_instruction() -> Result<()> {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let sender = Keypair::new();
    let destination_chain = "ethereum";
    let destination_address = hex::decode("2F43DDFf564Fb260dbD783D55fc6E4c70Be18862")?;
    let payload = rand_array::<32>().to_vec();
    let payload_hash = rand_array::<32>();

    let instruction = gateway::instructions::call_contract(
        gateway::id(),
        sender.pubkey(),
        destination_chain,
        &destination_address,
        &payload,
        payload_hash,
    )?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } = banks_client
        .process_transaction_with_metadata(transaction)
        .await?;

    assert!(result.is_ok(), "falied to process CallContract instruction");

    let expected_event = metadata
        .ok_or("expected transaction to have metadata")
        .unwrap()
        .log_messages
        .iter()
        .find_map(GatewayEvent::parse_log);

    assert_eq!(
        expected_event,
        Some(GatewayEvent::CallContract {
            sender: sender.pubkey().into(),
            destination_chain: destination_chain.as_bytes().to_vec(),
            destination_address,
            payload,
            payload_hash
        })
    );
    Ok(())
}

#[tokio::test]
async fn initialize_config() -> Result<()> {
    let (mut banks_client, payer, _recent_blockhash) = program_test().start().await;
    let gateway_config = GatewayConfig::new(1);
    accounts::initialize_config_account(&mut banks_client, &payer, &gateway_config).await
}

#[tokio::test]
async fn initialize_execute_data() -> Result<()> {
    let mut test_context = program_test().start_with_context().await;
    let execute_data = GatewayExecuteData::new(b"All you need is potatoes!".to_vec());
    let (execute_data_pda, _bump, _seeds) = execute_data.pda();
    accounts::initialize_execute_data_account(&mut test_context, execute_data_pda, execute_data)
        .await?;
    Ok(())
}

#[tokio::test]
async fn initialize_message() -> Result<()> {
    let (mut banks_client, payer, _recent_blockhash) = program_test().start().await;
    let approved_message = GatewayApprovedMessage::new([5; 32]);
    let (execute_data_pda, _bump, _seeds) = approved_message.pda();
    accounts::initialize_message(&mut banks_client, payer, execute_data_pda, approved_message)
        .await?;
    Ok(())
}

#[tokio::test]
async fn execute_message() -> Result<()> {
    let mut test_context = program_test().start_with_context().await;

    // Set up Gateway Config account
    let gateway_config = GatewayConfig::new(1);
    accounts::initialize_config_account(
        &mut test_context.banks_client,
        &test_context.payer,
        &gateway_config,
    )
    .await?;

    let execute_data_account = {
        let execute_data = GatewayExecuteData::new(b"All you need is potatoes!".to_vec());
        let (execute_data_pda, _bump, _seeds) = execute_data.pda();
        accounts::initialize_execute_data_account(
            &mut test_context,
            execute_data_pda,
            execute_data,
        )
        .await?;
        execute_data_pda
    };

    let message_accounts: Vec<Pubkey> = {
        // Be mindful of transaction max size and compute budget:
        // - Using more than 38 accounts causes the Address Lookup Table program reject
        //   the instruction with: 'invalid instruction data'.
        // - Using more than 14 accounts cause the `Execute` instruction to fail with
        //   'Program failed to complete'.
        let batch_size = 14;
        (0..batch_size)
            .map(|id| GatewayApprovedMessage::new([id; 32]).pda().0)
            .collect()
    };

    let mut slot_tracker = accounts::SlotTracker::new(&test_context, vec![10, 20, 30])?;

    let lookup_table_address = accounts::initialize_address_lookup_table(
        &mut test_context,
        &message_accounts,
        &mut slot_tracker,
    )
    .await?;

    let instruction =
        gateway::instructions::execute(gateway::ID, execute_data_account, &message_accounts)?;
    let transaction =
        helper::prepare_versioned_transaction(&mut test_context, instruction, lookup_table_address)
            .await?;

    test_context
        .banks_client
        .process_transaction(transaction)
        .await?;
    Ok(())
}

#[tokio::test]
async fn execute() -> Result<()> {
    let mut program_test = program_test();

    // Provision the test program with an `execute_data` account.

    // Copied from https://github.com/axelarnetwork/axelar-amplifier/blob/e04a85fc635448559cdd265da582d1f49a6b8f52/contracts/multisig-prover/src/encoding/bcs.rs#L509
    let execute_data = hex::decode("8a02010000000000000002000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020213617070726f7665436f6e747261637443616c6c13617070726f7665436f6e747261637443616c6c0249034554480330783000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000004c064158454c415203307831000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000087010121037286a4f1177bea06c8e15cf6ec3df0b7747a01ac2329ca2999dfd74eff59902801640000000000000000000000000000000a0000000000000000000000000000000141ef5ce016a4beed7e11761e5831805e962fca3d8901696a61a6ffd3af2b646bdc3740f64643bdb164b8151d1424eb4943d03f71e71816c00726e2d68ee55600c600")?;
    let execute_data_account = GatewayExecuteData::new(execute_data);
    let (execute_data_pda, _bump, _seeds) = execute_data_account.pda();
    let execute_data_base64 = STANDARD.encode(borsh::to_vec(&execute_data_account)?);

    program_test.add_account_with_base64_data(
        execute_data_pda,
        999999,
        gateway::id(),
        &execute_data_base64,
    );

    // Provision the test program with a Config account
    let config = GatewayConfig::new(1);
    let config_bytes = borsh::to_vec(&config)?;
    let config_base64 = STANDARD.encode(&config_bytes);
    let (config_pda, _bump) = find_root_pda();
    program_test.add_account_with_base64_data(config_pda, 999999, gateway::id(), &config_base64);

    // Provision the test progam with the message accounts.
    let message_pdas: Vec<Pubkey> = ([1, 2])
        .iter()
        .map(|suffix| {
            let mut id = [0u8; 32];
            id[31] = *suffix;
            let message = GatewayApprovedMessage::new(id);
            let message_pda = message.pda();
            message_pda.0
        })
        .collect();

    // Start the test program
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Prepare an `execute` instruction
    let instruction =
        gateway::instructions::execute(gateway::id(), execute_data_pda, &message_pdas)?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    let BanksTransactionResultWithMetadata { result, metadata } = banks_client
        .process_transaction_with_metadata(transaction)
        .await?;

    assert!(result.is_ok(), "failed to process Execute instruction");

    panic!("finish this test: check logs");

    Ok(())
}
