use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::Duration;

use axelar_rkyv_encoding::types::PublicKey;
use axelar_wasm_std::nonempty::Uint64;
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::tx::Msg;

use super::devnet_amplifier::{self};
use crate::cli::cmd::cosmwasm::cosmos_client::signer::SigningClient;
use crate::cli::cmd::cosmwasm::{default_gas, ResponseEventExtract};
use crate::cli::cmd::testnet::multisig_prover_api;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn wire_cosmwasm_contracts(
    source_chain_id: &str,
    destination_chain_id: &str,
    memo_to_send: String,
    message: &router_api::Message,
    cosmwasm_signer: SigningClient,
    source_axelar_gateway: &devnet_amplifier::Contract,
    source_axelar_voting_verifier: &devnet_amplifier::VotingVerifier,
    destination_multisig_prover: &devnet_amplifier::MultisigProver,
) -> eyre::Result<String> {
    tracing::info!(
        source = source_chain_id,
        dest = destination_chain_id,
        memo = memo_to_send,
        "memo sent"
    );
    tracing::info!("sleeping to allow the tx to settle");
    tokio::time::sleep(Duration::from_secs(10)).await;
    axelar_source_gateway_verify_messages(message, &cosmwasm_signer, source_axelar_gateway).await?;
    check_voting_verifier_status(message, &cosmwasm_signer, source_axelar_voting_verifier).await?;
    axelar_source_gateway_route_messages(message, &cosmwasm_signer, source_axelar_gateway).await?;
    let execute_data = axelar_destination_multisig_prover_construct_proof(
        message,
        destination_multisig_prover,
        cosmwasm_signer,
    )
    .await?;
    Ok(execute_data)
}

/// Workaround to temporarily skip multisig prover and rather construct the
/// proof off-chain
pub(crate) async fn wire_cosmwasm_contracts_without_destination_msig_prover(
    source_chain_id: &str,
    destination_chain_id: &str,
    memo_to_send: String,
    message: &router_api::Message,
    cosmwasm_signer: SigningClient,
    source_axelar_gateway: &devnet_amplifier::Contract,
    source_axelar_voting_verifier: &devnet_amplifier::VotingVerifier,
) -> eyre::Result<Vec<u8>> {
    tracing::info!(
        source = source_chain_id,
        dest = destination_chain_id,
        memo = memo_to_send,
        "memo sent"
    );
    tracing::info!("sleeping to allow the tx to settle");
    tokio::time::sleep(Duration::from_secs(10)).await;
    let signer = hex::decode("032dfbc482899a440b54fc203bac735bc4dc76d357192bf1cda1dc223bab6f60a2")
        .unwrap()
        .try_into()
        .unwrap();
    let mut signers = BTreeMap::new();
    signers.insert(PublicKey::new_ecdsa(signer), 1_u128.into());
    let verifier_set = axelar_rkyv_encoding::types::VerifierSet::new(1, signers, 1.into());

    axelar_source_gateway_verify_messages(message, &cosmwasm_signer, source_axelar_gateway).await?;
    check_voting_verifier_status(message, &cosmwasm_signer, source_axelar_voting_verifier).await?;
    axelar_source_gateway_route_messages(message, &cosmwasm_signer, source_axelar_gateway).await?;

    // we skip multisig prover and just construct our own data
    let signatures = vec![];
    let message = axelar_rkyv_encoding::types::Message::new(
        axelar_rkyv_encoding::types::CrossChainId::new(
            message.cc_id.chain.to_string(),
            message.cc_id.id.to_string(),
        ),
        message.source_address.to_string(),
        message.destination_chain.to_string(),
        message.destination_address.to_string(),
        message.payload_hash,
    );
    let execute_data = axelar_rkyv_encoding::encode::<0>(
        verifier_set.created_at(),
        *verifier_set.threshold(),
        signatures,
        axelar_rkyv_encoding::types::Payload::new_messages(vec![message.clone()]),
    )?;

    Ok(execute_data)
}

pub(crate) async fn check_voting_verifier_status(
    message: &router_api::Message,
    cosmwasm_signer: &SigningClient,
    voting_verifier: &devnet_amplifier::VotingVerifier,
) -> eyre::Result<()> {
    let vv_msg = voting_verifier::msg::QueryMsg::GetMessagesStatus {
        messages: vec![message.clone()],
    };
    let res = cosmwasm_signer
        .query::<serde_json::Value>(
            cosmrs::AccountId::from_str(&voting_verifier.address).unwrap(),
            serde_json::to_vec(&vv_msg).unwrap(),
        )
        .await?;
    tracing::info!(?res, "voting verifier status");
    Ok(())
}

pub(crate) async fn axelar_destination_multisig_prover_construct_proof(
    message: &router_api::Message,
    destination_multisig_prover: &devnet_amplifier::MultisigProver,
    cosmwasm_signer: SigningClient,
) -> eyre::Result<String> {
    tracing::info!("Axelar destination multisig_prover.construct_proof()");
    let msg = multisig_prover_api::MultisigProverExecuteMsg::ConstructProof {
        message_ids: vec![message.cc_id.clone()],
    };
    let destination_multisig_prover =
        cosmrs::AccountId::from_str(destination_multisig_prover.address.as_str()).unwrap();
    let execute = MsgExecuteContract {
        sender: cosmwasm_signer.signer_account_id()?,
        msg: serde_json::to_vec(&msg)?,
        funds: vec![],
        contract: destination_multisig_prover.clone(),
    };
    let response = cosmwasm_signer
        .sign_and_broadcast(vec![execute.into_any()?], &default_gas())
        .await?;
    let id = response.extract("wasm-proof_under_construction", "multisig_session_id")?;
    tracing::info!(multisig_session_id =? id, "found session id");
    let id = id.strip_prefix('"').ok_or(eyre::eyre!(
        "expected that the multisig_session_id is encapsulated in qutation marks"
    ))?;
    let id = id.strip_suffix('"').ok_or(eyre::eyre!(
        "expected that the multisig_session_id is encapsulated in qutation marks"
    ))?;
    let id: u64 = id.parse()?;
    let proof_response = loop {
        tracing::info!("attempting to get the proof for the multisig session");
        let res = cosmwasm_signer
            .query::<multisig_prover_api::GetProofResponse>(
                destination_multisig_prover.clone(),
                serde_json::to_vec(&multisig_prover_api::QueryMsg::GetProof {
                    multisig_session_id: Uint64::try_from(id)?,
                })?,
            )
            .await?;
        if res.status == multisig_prover_api::ProofStatus::Pending {
            tracing::info!("... still pending");
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }
        break res;
    };
    tracing::info!(?proof_response, "proof response");
    let multisig_prover_api::ProofStatus::Completed { execute_data } = proof_response.status else {
        eyre::bail!("status must be completed");
    };
    Ok(execute_data)
}

pub(crate) async fn axelar_source_gateway_route_messages(
    message: &router_api::Message,
    cosmwasm_signer: &SigningClient,
    soruce_axelar_gateway: &devnet_amplifier::Contract,
) -> eyre::Result<()> {
    tracing::info!("Axelar source Gateway.route_messages()");
    let msg = gateway_api::msg::ExecuteMsg::RouteMessages(vec![message.clone()]);
    let execute = MsgExecuteContract {
        sender: cosmwasm_signer.signer_account_id()?,
        msg: serde_json::to_vec(&msg)?,
        funds: vec![],
        contract: cosmrs::AccountId::from_str(soruce_axelar_gateway.address.as_str()).unwrap(),
    };
    let _response = cosmwasm_signer
        .sign_and_broadcast(vec![execute.into_any()?], &default_gas())
        .await?;

    Ok(())
}

pub(crate) async fn axelar_source_gateway_verify_messages(
    message: &router_api::Message,
    cosmwasm_signer: &SigningClient,
    soruce_axelar_gateway: &devnet_amplifier::Contract,
) -> eyre::Result<()> {
    tracing::info!(?message, "Axelar gateway.verify_messages");
    let msg = gateway_api::msg::ExecuteMsg::VerifyMessages(vec![message.clone()]);
    let execute = MsgExecuteContract {
        sender: cosmwasm_signer.signer_account_id()?,
        msg: serde_json::to_vec(&msg)?,
        funds: vec![],
        contract: cosmrs::AccountId::from_str(soruce_axelar_gateway.address.as_str()).unwrap(),
    };
    let _response = cosmwasm_signer
        .sign_and_broadcast(vec![execute.into_any()?], &default_gas())
        .await?;
    tracing::info!("sleeping for 30 seconds for the verifiers to respond");
    tokio::time::sleep(Duration::from_secs(30)).await;
    Ok(())
}
