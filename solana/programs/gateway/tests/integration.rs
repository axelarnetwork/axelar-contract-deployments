mod common;

use std::{assert, assert_eq};

use anyhow::{anyhow, bail, ensure, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use common::program_test;
use gmp_gateway::accounts::{GatewayApprovedMessage, GatewayConfig, GatewayExecuteData};
use gmp_gateway::events::GatewayEvent;
use gmp_gateway::get_gateway_root_config_pda;
use gmp_gateway::types::address::Address;
use gmp_gateway::types::bimap::OperatorsAndEpochs;
use gmp_gateway::types::execute_data_decoder::DecodedMessage;
use gmp_gateway::types::u256::U256;
use solana_program::keccak::hash;
use solana_program::pubkey::Pubkey;
use solana_program_test::{
    tokio, BanksClient, BanksTransactionResultWithMetadata, ProgramTestContext,
};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

mod accounts {
    use gmp_gateway::accounts::transfer_operatorship::TransferOperatorshipAccount;

    use super::*;
    pub(super) async fn initialize_config_account(
        client: &mut BanksClient,
        payer: &Keypair,
        gateway_config: &GatewayConfig,
    ) -> Result<()> {
        let recent_blockhash = client.get_latest_blockhash().await?;
        let (gateway_config_pda, _bump) = GatewayConfig::pda();

        let ix =
            gmp_gateway::instructions::initialize_config(payer.pubkey(), gateway_config.clone())?;
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

        assert_eq!(account.owner, gmp_gateway::id());
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
        let ix = gmp_gateway::instructions::initialize_execute_data(
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

        assert_eq!(account.owner, gmp_gateway::id());
        let deserialized_execute_data: GatewayExecuteData = borsh::from_slice(&account.data)?;
        assert_eq!(deserialized_execute_data, execute_data);

        Ok(())
    }

    pub(crate) async fn initialize_transfer_operatorship(
        client: &mut BanksClient,
        payer: Keypair,
        operators_and_weights: Vec<(Address, U256)>,
        threshold: U256,
    ) -> Result<()> {
        let recent_blockhash = client.get_latest_blockhash().await?;
        let ix = gmp_gateway::instructions::initialize_trasfer_operatorship(
            &payer.pubkey(),
            operators_and_weights.clone(),
            threshold,
        )?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        client.process_transaction(tx).await?;
        let expected_account = TransferOperatorshipAccount::new(operators_and_weights, threshold);
        let (pda, _bump) = expected_account.pda();
        let account = client.get_account(pda).await?.unwrap();
        assert_eq!(account.owner, gmp_gateway::ID);
        let deserialized_data: TransferOperatorshipAccount = borsh::from_slice(&account.data)?;
        assert_eq!(deserialized_data, expected_account);
        Ok(())
    }
}

#[tokio::test]
async fn test_call_contract_instruction() -> Result<()> {
    let (mut banks_client, sender, recent_blockhash) = program_test().start().await;
    let destination_chain = "ethereum";
    let destination_address = hex::decode("2F43DDFf564Fb260dbD783D55fc6E4c70Be18862")?;
    let payload = [1u8; 32].to_vec();
    let payload_hash = hash(&payload).to_bytes();

    let instruction = gmp_gateway::instructions::call_contract(
        gmp_gateway::id(),
        sender.pubkey(),
        destination_chain,
        &destination_address,
        &payload,
    )?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&sender.pubkey()),
        &[&sender],
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
    // TODO: try testing this without an empty operator set.
    let gateway_config = GatewayConfig::new(1, OperatorsAndEpochs::default());
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
async fn execute_with_axelar_provided_data() -> Result<()> {
    // Copied from https://github.com/axelarnetwork/axelar-amplifier/blob/e04a85fc635448559cdd265da582d1f49a6b8f52/contracts/multisig-prover/src/encoding/bcs.rs#L509
    let execute_data = hex::decode("8a02010000000000000002000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020213617070726f7665436f6e747261637443616c6c13617070726f7665436f6e747261637443616c6c0249034554480330783000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000004c064158454c415203307831000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000087010121037286a4f1177bea06c8e15cf6ec3df0b7747a01ac2329ca2999dfd74eff59902801640000000000000000000000000000000a0000000000000000000000000000000141ef5ce016a4beed7e11761e5831805e962fca3d8901696a61a6ffd3af2b646bdc3740f64643bdb164b8151d1424eb4943d03f71e71816c00726e2d68ee55600c600")?;
    execute(execute_data).await
}

#[tokio::test]
async fn execute_with_fixtures() -> Result<()> {
    use test_fixtures::execute_data::create_execute_data;

    // The current Gateway implementation exhausts the 200k Compute Units budget for
    // these combinations of Messages & Signers:
    // - one message and 8 signers
    // - 2 messages and 3 signers (sometimes it breaks at 4 signers)
    // - 3 messages and 2 signers
    // - 4 messages and one signer
    for m in 1..3 {
        for s in 1..4 {
            let execute_data = create_execute_data(m, s, 1)?;
            dbg!((m, s));
            execute(execute_data).await?;
        }
    }
    Ok(())
}

async fn execute(execute_data: Vec<u8>) -> Result<()> {
    // Setup
    let mut program_test = program_test();

    let (proof, command_batch) = gmp_gateway::types::execute_data_decoder::decode(&execute_data)?;
    let execute_data_account = GatewayExecuteData::new(execute_data);
    let (execute_data_pda, _bump, _seeds) = execute_data_account.pda();
    let execute_data_base64 = STANDARD.encode(borsh::to_vec(&execute_data_account)?);
    let allowed_executioner = Keypair::new();

    program_test.add_account_with_base64_data(
        execute_data_pda,
        999999,
        gmp_gateway::id(),
        &execute_data_base64,
    );

    // Provision the test program with a Config account
    let mut config = GatewayConfig::new(1, OperatorsAndEpochs::default());
    config.operators_and_epochs.update(proof.operators_hash())?;
    let config_bytes = borsh::to_vec(&config)?;
    let config_base64 = STANDARD.encode(&config_bytes);
    let (gateway_root_pda, _bump) = get_gateway_root_config_pda();
    program_test.add_account_with_base64_data(
        gateway_root_pda,
        999999,
        gmp_gateway::id(),
        &config_base64,
    );

    // Provision the test program with the message accounts.
    let mut message_pdas: Vec<Pubkey> = vec![];
    let pending_message_account_base64 = STANDARD.encode(borsh::to_vec(
        &GatewayApprovedMessage::pending(allowed_executioner.pubkey()),
    )?);
    for command in &command_batch.commands {
        let approved_message_pda =
            GatewayApprovedMessage::pda_from_decoded_command(gateway_root_pda, command);
        program_test.add_account_with_base64_data(
            approved_message_pda,
            999999,
            gmp_gateway::id(),
            &pending_message_account_base64,
        );
        message_pdas.push(approved_message_pda);
    }

    // Start the test program
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Prepare an `execute` instruction
    let instruction = gmp_gateway::instructions::execute(
        gmp_gateway::id(),
        execute_data_pda,
        gateway_root_pda,
        &message_pdas,
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

    assert!(result.is_ok(), "failed to process Execute instruction");

    // Check if every approved message account data was updated to 'Approved'.
    for approved_message_address in &message_pdas {
        let approved_message_account = banks_client
            .get_account(*approved_message_address)
            .await?
            .expect("the account we created earlier");
        let approved_message: GatewayApprovedMessage =
            borsh::from_slice(&approved_message_account.data)?;
        assert!(approved_message.is_approved());
    }

    // Check if the expected logs were emitted.
    let mut events_logged = 0;
    metadata
        .ok_or(anyhow!("expected metadata"))?
        .log_messages
        .iter()
        .filter_map(GatewayEvent::parse_log)
        .filter(|event| matches!(event, GatewayEvent::MessageApproved { .. }))
        .zip(
            command_batch
                .commands
                .iter()
                .map(|command| &command.message),
        )
        .try_for_each(|(event, message)| {
            events_logged += 1;
            ensure_message_approved_event_matches_decoded_message(&event, message)
                .map_err(|err| anyhow!("Wrong event emitted: {err}"))
        })?;

    assert_eq!(
        events_logged,
        command_batch.commands.len(),
        "Not all approved messages resulted in events being emitted"
    );
    Ok(())
}

fn ensure_message_approved_event_matches_decoded_message(
    event: &GatewayEvent,
    message: &DecodedMessage,
) -> Result<()> {
    let GatewayEvent::MessageApproved {
        message_id,
        source_chain,
        source_address,
        destination_address,
        payload_hash,
    } = event
    else {
        bail!("Wrong type of event")
    };
    ensure!(*message_id == message.id, "Wrong message id");
    ensure!(*source_chain == message.source_chain, "Wrong source chain");
    ensure!(
        *source_address == message.source_address,
        "Wrong source address"
    );
    ensure!(
        *destination_address == message.destination_address,
        "Wrong destination address"
    );
    ensure!(*payload_hash == message.payload_hash, "Wrong payload hash");
    Ok(())
}

#[tokio::test]
async fn initialize_transfer_operatorship() -> Result<()> {
    use test_fixtures::primitives::{array32, bytes};
    let (mut banks_client, payer, _recent_blockhash) = program_test().start().await;

    let mut operators_and_weights = vec![];
    for _ in 0..3 {
        let operator = Address::try_from(&*bytes(33))?;
        let weight = U256::from_le_bytes(array32());
        operators_and_weights.push((operator, weight))
    }
    let threshold = U256::from_le_bytes(array32());

    accounts::initialize_transfer_operatorship(
        &mut banks_client,
        payer,
        operators_and_weights,
        threshold,
    )
    .await?;
    Ok(())
}
