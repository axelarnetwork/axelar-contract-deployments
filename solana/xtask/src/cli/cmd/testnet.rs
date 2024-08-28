pub(crate) mod cosmwasm_interactions;
pub(crate) mod evm_interaction;
pub(crate) mod multisig_prover_api;
pub(crate) mod solana_interactions;

use std::time::Duration;

use axelar_message_primitives::DataPayload;
use ethers::types::Address as EvmAddress;
use evm_contracts_test_suite::EvmSigner;
use gmp_gateway::hasher_impl;
use solana_sdk::signature::Keypair;

use self::devnet_amplifier::EvmChain;
use super::cosmwasm::cosmos_client::signer::SigningClient;
use super::cosmwasm::domain_separator;
use crate::cli::cmd::evm::{send_memo_from_evm_to_evm, send_memo_to_solana};

pub(crate) const SOLANA_CHAIN_NAME: &str = "solana-devnet";
pub(crate) const SOLANA_CHAIN_ID: u64 = 43113;

pub(crate) fn solana_domain_separator() -> [u8; 32] {
    domain_separator(SOLANA_CHAIN_NAME, SOLANA_CHAIN_ID)
}

fn solana_axelar_voting_verifier() -> devnet_amplifier::VotingVerifier {
    devnet_amplifier::VotingVerifier {
        governance_address: "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9".to_string(),
        source_gateway_address: "gtwrtxmhBP2TCXV1SgDQ6FijJkuuXyWoP2aFqaPcwhj".to_string(),
        address: "axelar1qsvct6yu0dmx73axhsrjkrd9606jhkh35wfj8ernkdde6864yecszv8s6p".to_string(),
        msg_id_format: "base58".to_string(),
    }
}

fn solana_axelar_gateway() -> devnet_amplifier::Contract {
    devnet_amplifier::Contract {
        address: "axelar12yhem4kvpk7lsny8250z8k5n0p7wuqewjzyyah66jg3y2rjgdxfssej9wn".to_string(),
    }
}

fn solana_axelar_multisig_prover() -> devnet_amplifier::MultisigProver {
    devnet_amplifier::MultisigProver {
        governance_address: "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9".to_string(),
        destination_chain_id: SOLANA_CHAIN_NAME.to_string(),
        service_name: "validators".to_string(),
        encoder: "rkyv".to_string(),
        address: "axelar1y7vkqzms5vqt0m0lx95ylh9upc0l552vzyhfwnnc3wajyaj6g5ysf03420".to_string(),
        domain_separator: hex::encode(solana_domain_separator()),
        key_type: "ecdsa".to_string(),
    }
}

#[allow(clippy::all, clippy::pedantic, warnings, clippy::unreadable_literal)]
pub(crate) mod devnet_amplifier {
    include!(concat!(env!("OUT_DIR"), "/devnet_amplifier.rs"));
}

pub(crate) async fn evm_to_solana(
    source_chain: &EvmChain,
    source_evm_signer: EvmSigner,
    cosmwasm_signer: SigningClient,
    source_memo_contract: EvmAddress,
    solana_rpc_client: solana_client::rpc_client::RpcClient,
    solana_keypair: Keypair,
    memo_to_send: String,
) -> eyre::Result<()> {
    let destination_chain_name = SOLANA_CHAIN_NAME;
    let axelar_cosmwasm = devnet_amplifier::get_axelar();
    let source_axelar_gateway = axelar_cosmwasm
        .gateway
        .get(source_chain.id.as_str())
        .unwrap();
    let source_axelar_voting_verifier = axelar_cosmwasm
        .voting_verifier
        .get(source_chain.id.as_str())
        .unwrap();
    let destination_multisig_prover = solana_axelar_multisig_prover();

    let root_pda = gmp_gateway::get_gateway_root_config_pda().0;
    let root_pda = solana_rpc_client.get_account(&root_pda).unwrap();
    let account_data =
        borsh::from_slice::<gmp_gateway::state::GatewayConfig>(&root_pda.data).unwrap();
    tracing::info!(?account_data, "solana gateway root pda config");

    let tx = send_memo_to_solana(
        source_evm_signer,
        source_memo_contract,
        memo_to_send.as_str(),
        destination_chain_name,
    )
    .await?;
    tracing::info!(
        source = source_chain.id,
        dest = destination_chain_name,
        memo = memo_to_send,
        "memo sent"
    );
    tracing::info!("sleeping to allow the tx to settle");
    tokio::time::sleep(Duration::from_secs(30)).await;
    let (payload, message) = evm_interaction::create_axelar_message_from_evm_log(&tx, source_chain);
    let decoded_payload = DataPayload::decode(payload.0.as_ref()).unwrap();
    let execute_data = cosmwasm_interactions::wire_cosmwasm_contracts(
        source_chain.id.as_str(),
        destination_chain_name,
        memo_to_send,
        &message,
        cosmwasm_signer,
        source_axelar_gateway,
        source_axelar_voting_verifier,
        &destination_multisig_prover,
    )
    .await?;
    let gateway_root_pda = gmp_gateway::get_gateway_root_config_pda().0;
    let decoded_execute_data =
        axelar_rkyv_encoding::types::ExecuteData::from_bytes(&execute_data).unwrap();
    let signing_verifier_set = decoded_execute_data.proof.verifier_set();
    let (signing_verifier_set_pda, _) = gmp_gateway::get_verifier_set_tracker_pda(
        &gmp_gateway::id(),
        signing_verifier_set.hash(hasher_impl()),
    );

    // solana: initialize pending command pdas
    let (gateway_approved_message_pda, message) = solana_interactions::solana_init_approved_command(
        &gateway_root_pda,
        &message,
        &solana_keypair,
        &solana_rpc_client,
    );

    // update execute data
    let execute_data_pda = solana_interactions::solana_init_approve_messages_execute_data(
        &solana_keypair,
        gateway_root_pda,
        &execute_data,
        &solana_rpc_client,
    );

    // call `approve messages`
    solana_interactions::solana_approve_messages(
        execute_data_pda,
        gateway_root_pda,
        gateway_approved_message_pda,
        signing_verifier_set_pda,
        &solana_rpc_client,
        &solana_keypair,
    );

    // call the destination memo program
    solana_interactions::solana_call_executable(
        message,
        &decoded_payload,
        gateway_approved_message_pda,
        gateway_root_pda,
        &solana_rpc_client,
        &solana_keypair,
    );

    let (counter_pda, _counter_bump) =
        axelar_solana_memo_program::get_counter_pda(&gateway_root_pda);
    let acc = solana_rpc_client.get_account(&counter_pda).unwrap();
    let acc = borsh::from_slice::<axelar_solana_memo_program::state::Counter>(&acc.data).unwrap();
    tracing::info!(counter_pda =? acc, "counter PDA");
    Ok(())
}

pub(crate) async fn solana_to_evm(
    destination_chain: &EvmChain,
    destination_evm_signer: EvmSigner,
    cosmwasm_signer: SigningClient,
    destination_memo_contract: EvmAddress,
    solana_rpc_client: solana_client::rpc_client::RpcClient,
    solana_keypair: Keypair,
    memo_to_send: String,
) -> eyre::Result<()> {
    let source_chain_name = SOLANA_CHAIN_NAME;
    let source_axelar_gateway = solana_axelar_gateway();
    let axelar_cosmwasm = devnet_amplifier::get_axelar();
    let destination_multisig_prover = axelar_cosmwasm
        .multisig_prover
        .get(destination_chain.id.as_str())
        .unwrap();
    let source_axelar_voting_verifier = solana_axelar_voting_verifier();

    let gateway_root_pda = gmp_gateway::get_gateway_root_config_pda().0;
    let (payload, message) = solana_interactions::send_memo_from_solana(
        &solana_rpc_client,
        &gateway_root_pda,
        &solana_keypair,
        destination_chain,
        source_chain_name,
        destination_memo_contract,
        memo_to_send.as_str(),
    );
    let execute_data = cosmwasm_interactions::wire_cosmwasm_contracts(
        source_chain_name,
        &destination_chain.id,
        memo_to_send,
        &message,
        cosmwasm_signer,
        &source_axelar_gateway,
        &source_axelar_voting_verifier,
        destination_multisig_prover,
    )
    .await?;

    // Call the EVM contracts
    evm_interaction::approve_messages_on_evm_gateway(
        destination_chain,
        execute_data,
        &destination_evm_signer,
    )
    .await?;
    evm_interaction::call_execute_on_destination_evm_contract(
        message,
        destination_memo_contract,
        destination_evm_signer,
        payload.iter().collect::<ethers::types::Bytes>(),
    )
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn evm_to_evm(
    source_chain: &devnet_amplifier::EvmChain,
    destination_chain: &devnet_amplifier::EvmChain,
    source_memo_contract: EvmAddress,
    destination_memo_contract: EvmAddress,
    source_evm_signer: EvmSigner,
    destination_evm_signer: EvmSigner,
    memo_to_send: String,
    cosmwasm_signer: SigningClient,
) -> eyre::Result<()> {
    let axelar_cosmwasm = devnet_amplifier::get_axelar();
    let source_axelar_gateway = axelar_cosmwasm
        .gateway
        .get(source_chain.id.as_str())
        .unwrap();
    let source_axelar_voting_verifier = axelar_cosmwasm
        .voting_verifier
        .get(source_chain.id.as_str())
        .unwrap();
    let destination_multisig_prover = axelar_cosmwasm
        .multisig_prover
        .get(destination_chain.id.as_str())
        .unwrap();

    // let destination_memo_contract =
    // ethers::utils::to_checksum(&destination_memo_contract, None);
    let tx = send_memo_from_evm_to_evm(
        source_evm_signer,
        source_memo_contract,
        memo_to_send.clone(),
        destination_chain.id.clone(),
        ethers::utils::to_checksum(&destination_memo_contract, None),
    )
    .await?;
    tracing::info!(
        source = source_chain.id,
        dest = destination_chain.id,
        memo = memo_to_send,
        "memo sent"
    );
    tracing::info!("sleeping to allow the tx to settle");
    tokio::time::sleep(Duration::from_secs(30)).await;
    let (payload, message) = evm_interaction::create_axelar_message_from_evm_log(&tx, source_chain);

    let execute_data = cosmwasm_interactions::wire_cosmwasm_contracts(
        source_chain.id.as_str(),
        &destination_chain.id,
        memo_to_send,
        &message,
        cosmwasm_signer,
        source_axelar_gateway,
        source_axelar_voting_verifier,
        destination_multisig_prover,
    )
    .await?;

    // Call the destination chain Gateway
    evm_interaction::approve_messages_on_evm_gateway(
        destination_chain,
        execute_data,
        &destination_evm_signer,
    )
    .await?;
    evm_interaction::call_execute_on_destination_evm_contract(
        message,
        destination_memo_contract,
        destination_evm_signer,
        payload,
    )
    .await?;

    Ok(())
}
