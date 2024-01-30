// #![cfg(feature = "test-sbf")]
mod common;

use anyhow::{anyhow, bail, ensure, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use common::program_test;
use connection_router::Message as AxelarMessage;
use gateway::accounts::{GatewayApprovedMessage, GatewayConfig, GatewayExecuteData};
use gateway::events::GatewayEvent;
use gateway::find_root_pda;
use gateway::types::bimap::OperatorsAndEpochs;
use gateway::types::execute_data_decoder::DecodedMessage;
use solana_program::hash::hash;
use solana_program::pubkey::Pubkey;
use solana_program_test::{
    tokio, BanksClient, BanksTransactionResultWithMetadata, ProgramTestContext,
};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

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
        message_id: [u8; 32],
        source_chain: &[u8],
        source_address: &[u8],
        payload_hash: [u8; 32],
    ) -> Result<()> {
        let recent_blockhash = client.get_latest_blockhash().await?;
        let ix = gateway::instructions::initialize_messge(
            payer.pubkey(),
            message_id,
            source_chain,
            source_address,
            payload_hash,
        )?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        client.process_transaction(tx).await?;

        let (pda, _bump) =
            GatewayApprovedMessage::pda(message_id, source_chain, source_address, payload_hash);

        let account = client.get_account(pda).await?.expect("metadata");
        assert_eq!(account.owner, gateway::id());
        let deserialized_execute_data: GatewayApprovedMessage = borsh::from_slice(&account.data)?;
        assert_eq!(deserialized_execute_data, GatewayApprovedMessage::pending());

        Ok(())
    }
}

#[tokio::test]
async fn test_call_contract_instruction() -> Result<()> {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let sender = Keypair::new();
    let destination_chain = "ethereum";
    let destination_address = hex::decode("2F43DDFf564Fb260dbD783D55fc6E4c70Be18862")?;
    let payload = test_fixtures::primitives::array32().to_vec();
    let payload_hash = test_fixtures::primitives::array32();

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
async fn initialize_message() -> Result<()> {
    let (mut banks_client, payer, _recent_blockhash) = program_test().start().await;

    let AxelarMessage {
        cc_id,
        source_address,
        payload_hash,
        ..
    } = test_fixtures::axelar_message::message()?;

    // We hash it the same way as `[multisig_prover]` does during encoding.
    let message_id = hash(cc_id.to_string().as_bytes()).to_bytes();

    accounts::initialize_message(
        &mut banks_client,
        payer,
        message_id,
        cc_id.to_string().as_bytes(),
        source_address.as_bytes(),
        payload_hash,
    )
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
    // - one message and 15 signers
    // - 2 messages and 9 signers
    // - 3 messages and 4 signers (sometimes it breaks at 5 signers)
    // - 4 messages and one signer
    for m in 1..4 {
        for s in 1..4 {
            println!("Messages = {}, Signers = {}", m, s);
            let execute_data = create_execute_data(m, s, 1)?;
            execute(execute_data).await?;
        }
    }

    Ok(())
}

async fn execute(execute_data: Vec<u8>) -> Result<()> {
    let mut program_test = program_test();

    // Provision the test program with an `execute_data` account.

    let (_proof, command_batch) = gateway::types::execute_data_decoder::decode(&execute_data)?;
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
    // TODO: the final version of the `execute` instruction won't work with an empty
    // operator set.
    let config = GatewayConfig::new(1, OperatorsAndEpochs::default());
    let config_bytes = borsh::to_vec(&config)?;
    let config_base64 = STANDARD.encode(&config_bytes);
    let (config_pda, _bump) = find_root_pda();
    program_test.add_account_with_base64_data(config_pda, 999999, gateway::id(), &config_base64);

    // Provision the test progam with the message accounts.
    let mut message_pdas: Vec<Pubkey> = vec![];
    let pending_message_account_base64 =
        STANDARD.encode(borsh::to_vec(&GatewayApprovedMessage::pending())?);
    for command in &command_batch.commands {
        let DecodedMessage {
            id,
            source_chain,
            source_address,
            payload_hash,
            ..
        } = &command.message;
        let (approved_message_pda, _bump) = GatewayApprovedMessage::pda(
            *id,
            source_chain.as_bytes(),
            source_address.as_bytes(),
            *payload_hash,
        );
        program_test.add_account_with_base64_data(
            approved_message_pda,
            999999,
            gateway::id(),
            &pending_message_account_base64,
        );
        message_pdas.push(approved_message_pda);
    }

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
