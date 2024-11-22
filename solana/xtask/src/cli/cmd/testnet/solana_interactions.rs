use std::str::FromStr;

use axelar_message_primitives::DataPayload;
use eyre::OptionExt;
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::events::ArchivedGatewayEvent;
use gmp_gateway::state::GatewayApprovedCommand;
use router_api::{Address, ChainName, CrossChainId};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_transaction_status::UiTransactionEncoding;

use crate::cli::cmd::axelar_deployments::EvmChain;
use crate::cli::cmd::deployments::SolanaConfiguration;

pub(crate) fn send_memo_from_solana(
    solana_rpc_client: &solana_client::rpc_client::RpcClient,
    gateway_root_pda: &solana_sdk::pubkey::Pubkey,
    solana_keypair: &Keypair,
    destination_chain: &EvmChain,
    solana_chain_id: &str,
    destination_memo_contract: ethers::types::H160,
    memo: &str,
) -> eyre::Result<(Vec<u8>, router_api::Message)> {
    let hash = solana_rpc_client.get_latest_blockhash()?;
    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[
            axelar_solana_memo_program::instruction::call_gateway_with_memo(
                gateway_root_pda,
                &solana_keypair.pubkey(),
                memo.to_string(),
                destination_chain.id.clone(),
                ethers::utils::to_checksum(&destination_memo_contract, None),
                &gmp_gateway::ID,
            )?,
        ],
        Some(&solana_keypair.pubkey()),
        &[&solana_keypair],
        hash,
    );
    let signature = solana_rpc_client.send_and_confirm_transaction(&tx)?;

    // Fetch the transaction details using the signature
    let tx_details = solana_rpc_client.get_transaction_with_config(
        &signature,
        solana_client::rpc_config::RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Json),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: None,
        },
    )?;

    // Extract log messages from the transaction metadata
    let log_msgs = tx_details
        .transaction
        .meta
        .ok_or_eyre("no meta field")?
        .log_messages
        .ok_or(eyre::eyre!("no log messages"))?;

    for log in &log_msgs {
        tracing::info!(?log, "solana tx log");
    }
    let (event_idx, gateway_event) = log_msgs
        .iter()
        .enumerate()
        .find_map(|(idx, log)| gmp_gateway::events::GatewayEvent::parse_log(log).map(|x| (idx, x)))
        .expect("Gateway event was not emitted (or we couldn't parse it)?");
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };
    let payload = call_contract.payload.to_vec();
    let signature = signature.to_string();
    let message = router_api::Message {
        cc_id: CrossChainId::new(solana_chain_id, format!("{signature}-{event_idx}")).unwrap(),
        source_address: Address::from_str(
            solana_sdk::pubkey::Pubkey::from(call_contract.sender)
                .to_string()
                .as_str(),
        )
        .map_err(|err| eyre::eyre!(format!("invalid pubkey: {}", err.to_string())))?,
        destination_chain: ChainName::from_str(call_contract.destination_chain.as_str())
            .map_err(|err| eyre::eyre!(format!("{}", err.to_string())))?,
        destination_address: Address::from_str(call_contract.destination_address.as_str())
            .map_err(|err| eyre::eyre!(format!("{}", err.to_string())))?,
        payload_hash: call_contract.payload_hash,
    };

    Ok((payload, message))
}

#[tracing::instrument(skip_all)]
pub(crate) fn solana_call_executable(
    message: axelar_rkyv_encoding::types::Message,
    decoded_payload: &DataPayload<'_>,
    gateway_approved_message_pda: solana_sdk::pubkey::Pubkey,
    gateway_root_pda: solana_sdk::pubkey::Pubkey,
    solana_rpc_client: &solana_client::rpc_client::RpcClient,
    solana_keypair: &Keypair,
) -> eyre::Result<()> {
    tracing::info!(payload = ?decoded_payload, "call the destination program");

    let ix = axelar_executable_old::construct_axelar_executable_ix(
        message,
        decoded_payload.encode()?,
        gateway_approved_message_pda,
        gateway_root_pda,
    )?;

    send_solana_tx(
        solana_rpc_client,
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_399_850_u32),
            ix,
        ],
        solana_keypair,
    )?;
    let acc = solana_rpc_client.get_account(&gateway_approved_message_pda)?;
    let acc = borsh::from_slice::<gmp_gateway::state::GatewayApprovedCommand>(&acc.data)?;
    tracing::info!(?acc, "approved command status");
    Ok(())
}

#[tracing::instrument(skip_all)]
pub(crate) fn solana_init_approve_messages_execute_data(
    solana_keypair: &Keypair,
    gateway_root_pda: solana_sdk::pubkey::Pubkey,
    execute_data: &[u8],
    solana_rpc_client: &solana_client::rpc_client::RpcClient,
    solana_config: &SolanaConfiguration,
) -> eyre::Result<solana_sdk::pubkey::Pubkey> {
    tracing::info!("solana gateway.initialize_execute_data()");
    let (ix, execute_data) = gmp_gateway::instructions::initialize_approve_messages_execute_data(
        solana_keypair.pubkey(),
        gateway_root_pda,
        &solana_config.domain_separator,
        execute_data,
    )?;
    let (execute_data_pda, ..) =
        gmp_gateway::get_execute_data_pda(&gateway_root_pda, &execute_data.hash_decoded_contents());
    tracing::info!(?execute_data_pda, "execute data pda");
    send_solana_tx(
        solana_rpc_client,
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_399_850_u32),
            ix,
        ],
        solana_keypair,
    )?;
    Ok(execute_data_pda)
}

#[tracing::instrument(skip_all)]
pub(crate) fn solana_init_rotate_signers_execute_data(
    solana_keypair: &Keypair,
    gateway_root_pda: solana_sdk::pubkey::Pubkey,
    execute_data: &[u8],
    solana_rpc_client: &solana_client::rpc_client::RpcClient,
    solana_config: &SolanaConfiguration,
) -> eyre::Result<solana_sdk::pubkey::Pubkey> {
    tracing::info!("solana gateway.initialize_execute_data()");
    let (ix, execute_data) = gmp_gateway::instructions::initialize_rotate_signers_execute_data(
        solana_keypair.pubkey(),
        gateway_root_pda,
        &solana_config.domain_separator,
        execute_data,
    )?;
    let (execute_data_pda, ..) =
        gmp_gateway::get_execute_data_pda(&gateway_root_pda, &execute_data.hash_decoded_contents());
    tracing::info!(?execute_data_pda, "execute data pda");
    send_solana_tx(
        solana_rpc_client,
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_399_850_u32),
            ix,
        ],
        solana_keypair,
    )?;
    Ok(execute_data_pda)
}

#[tracing::instrument(skip_all)]
pub(crate) fn solana_approve_messages(
    execute_data_pda: solana_sdk::pubkey::Pubkey,
    gateway_root_pda: solana_sdk::pubkey::Pubkey,
    gateway_approved_message_pda: solana_sdk::pubkey::Pubkey,
    verifier_set_tracker_pda: solana_sdk::pubkey::Pubkey,
    solana_rpc_client: &solana_client::rpc_client::RpcClient,
    solana_keypair: &Keypair,
) -> eyre::Result<()> {
    tracing::info!("solana gateway.approve_messages()");
    let ix = gmp_gateway::instructions::approve_messages(
        execute_data_pda,
        gateway_root_pda,
        &[gateway_approved_message_pda],
        verifier_set_tracker_pda,
    )?;

    send_solana_tx(
        solana_rpc_client,
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_399_850_u32),
            ix,
        ],
        solana_keypair,
    )?;
    let acc = solana_rpc_client.get_account(&gateway_approved_message_pda)?;
    let acc = borsh::from_slice::<gmp_gateway::state::GatewayApprovedCommand>(&acc.data)?;
    tracing::info!(?acc, "approved command");
    Ok(())
}

#[tracing::instrument(skip_all)]
pub(crate) fn solana_init_approved_command(
    gateway_root_pda: &solana_sdk::pubkey::Pubkey,
    message: &router_api::Message,
    solana_keypair: &Keypair,
    solana_rpc_client: &solana_client::rpc_client::RpcClient,
    solana_config: &SolanaConfiguration,
) -> eyre::Result<(
    solana_sdk::pubkey::Pubkey,
    axelar_rkyv_encoding::types::Message,
)> {
    tracing::info!("solana gateway.initialize_commands()");
    let message = axelar_rkyv_encoding::types::Message::new(
        axelar_rkyv_encoding::types::CrossChainId::new(
            message.cc_id.source_chain.to_string(),
            message.cc_id.message_id.to_string(),
        ),
        message.source_address.to_string(),
        message.destination_chain.to_string(),
        message.destination_address.to_string(),
        message.payload_hash,
        solana_config.domain_separator,
    );
    let command = OwnedCommand::ApproveMessage(message.clone());
    let (gateway_approved_message_pda, _bump, _seeds) =
        GatewayApprovedCommand::pda(gateway_root_pda, &command);
    let ix = gmp_gateway::instructions::initialize_pending_command(
        gateway_root_pda,
        &solana_keypair.pubkey(),
        command.clone(),
    )?;
    send_solana_tx(solana_rpc_client, &[ix], solana_keypair)?;
    let acc = solana_rpc_client.get_account(&gateway_approved_message_pda)?;
    let acc = borsh::from_slice::<gmp_gateway::state::GatewayApprovedCommand>(&acc.data)?;
    tracing::info!(?acc, "approved command status");
    Ok((gateway_approved_message_pda, message))
}

pub(crate) fn send_solana_tx(
    solana_rpc_client: &solana_client::rpc_client::RpcClient,
    ixs: &[solana_sdk::instruction::Instruction],
    solana_keypair: &Keypair,
) -> eyre::Result<()> {
    let hash = solana_rpc_client.get_latest_blockhash()?;
    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        ixs,
        Some(&solana_keypair.pubkey()),
        &[solana_keypair],
        hash,
    );
    let signature = solana_rpc_client.send_and_confirm_transaction_with_spinner(&tx)?;
    let devnet_url = format!("https://explorer.solana.com/tx/{signature:?}?cluster=devnet",);
    tracing::info!(?signature, devnet_url, "solana tx sent");
    Ok(())
}
