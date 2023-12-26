// #![cfg(feature = "test-sbf")]

mod common;

use anyhow::{anyhow, Result};
use common::program_test;
use gateway::accounts::{GatewayConfig, GatewayExecuteData, GatewayMessageID};
use gateway::events::GatewayEvent;
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
        payer: Keypair,
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
        message_id: GatewayMessageID,
    ) -> Result<()> {
        let recent_blockhash = client.get_latest_blockhash().await?;
        let ix = gateway::instructions::initialize_messge(payer.pubkey(), pda, message_id.clone())?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        client.process_transaction(tx).await?;

        let account = client.get_account(pda).await?.expect("metadata");
        assert_eq!(account.owner, gateway::id());
        let deserialized_execute_data: GatewayMessageID = borsh::from_slice(&account.data)?;
        assert_eq!(deserialized_execute_data, message_id);

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

    assert!({ result.is_ok() });

    let expected_event = metadata
        .ok_or("expected transaction to have metadata")
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GatewayEvent::parse_log)
        .next();

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
    accounts::initialize_config_account(&mut banks_client, payer, &gateway_config).await
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
    let message_id = GatewayMessageID::new("All you need is potatoes!".into());
    let (execute_data_pda, _bump, _seeds) = message_id.pda();
    accounts::initialize_message(&mut banks_client, payer, execute_data_pda, message_id).await?;
    Ok(())
}

#[tokio::test]
async fn execute_message() -> Result<()> {
    let mut test_context = program_test().start_with_context().await;

    // TODO: define a data type for the execute_data bytes produced by andreceived
    // from the Prover.
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
            .map(|id| GatewayMessageID::new(format!("message{id}")).pda().0)
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
