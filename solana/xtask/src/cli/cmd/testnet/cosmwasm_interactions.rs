use std::time::Duration;

use axelar_wasm_std::nonempty::Uint64;
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::tx::Msg;

use crate::cli::cmd::cosmwasm::cosmos_client::signer::SigningClient;
use crate::cli::cmd::cosmwasm::{default_gas, ResponseEventExtract};
use crate::cli::cmd::deployments::AxelarConfiguration;
use crate::cli::cmd::testnet::multisig_prover_api;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn wire_cosmwasm_contracts(
    source_chain_id: &str,
    destination_chain_id: &str,
    memo_to_send: String,
    message: &router_api::Message,
    cosmwasm_signer: SigningClient,
    source_axelar_gateway: &cosmrs::AccountId,
    source_axelar_voting_verifier: &cosmrs::AccountId,
    destination_multisig_prover: &cosmrs::AccountId,
    axelar_config: &AxelarConfiguration,
) -> eyre::Result<Vec<u8>> {
    tracing::info!(
        source = source_chain_id,
        dest = destination_chain_id,
        memo = memo_to_send,
        "memo sent"
    );
    tracing::info!("sleeping to allow the tx to settle");
    tokio::time::sleep(Duration::from_secs(10)).await;
    axelar_source_gateway_verify_messages(
        message,
        &cosmwasm_signer,
        source_axelar_gateway,
        axelar_config,
    )
    .await?;
    check_voting_verifier_status(
        message,
        &cosmwasm_signer,
        source_axelar_voting_verifier.clone(),
    )
    .await?;
    axelar_source_gateway_route_messages(
        message,
        &cosmwasm_signer,
        source_axelar_gateway,
        axelar_config,
    )
    .await?;
    let execute_data = axelar_destination_multisig_prover_construct_proof(
        message,
        destination_multisig_prover,
        cosmwasm_signer,
        axelar_config,
    )
    .await?;
    let execute_data = hex::decode(execute_data.as_str())?;
    Ok(execute_data)
}

pub(crate) async fn check_voting_verifier_status(
    message: &router_api::Message,
    cosmwasm_signer: &SigningClient,
    voting_verifier: cosmrs::AccountId,
) -> eyre::Result<()> {
    let vv_msg = voting_verifier::msg::QueryMsg::GetMessagesStatus {
        messages: vec![message.clone()],
    };
    let res = cosmwasm_signer
        .query::<serde_json::Value>(voting_verifier, serde_json::to_vec(&vv_msg).unwrap())
        .await?;
    tracing::info!(?res, "voting verifier status");
    Ok(())
}

pub(crate) async fn axelar_destination_multisig_prover_construct_proof(
    message: &router_api::Message,
    destination_multisig_prover: &cosmrs::AccountId,
    cosmwasm_signer: SigningClient,
    config: &AxelarConfiguration,
) -> eyre::Result<String> {
    tracing::info!("Axelar destination multisig_prover.construct_proof()");
    let msg = multisig_prover_api::MultisigProverExecuteMsg::ConstructProof {
        message_ids: vec![message.cc_id.clone()],
    };
    let execute = MsgExecuteContract {
        sender: cosmwasm_signer.signer_account_id()?,
        msg: serde_json::to_vec(&msg)?,
        funds: vec![],
        contract: destination_multisig_prover.clone(),
    };
    let response = cosmwasm_signer
        .sign_and_broadcast(vec![execute.into_any()?], &default_gas(config)?)
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
    source_axelar_gateway: &cosmrs::AccountId,
    config: &AxelarConfiguration,
) -> eyre::Result<()> {
    tracing::info!("Axelar source Gateway.route_messages()");
    let msg = gateway_api::msg::ExecuteMsg::RouteMessages(vec![message.clone()]);
    let execute = MsgExecuteContract {
        sender: cosmwasm_signer.signer_account_id()?,
        msg: serde_json::to_vec(&msg)?,
        funds: vec![],
        contract: source_axelar_gateway.clone(),
    };
    let _response = cosmwasm_signer
        .sign_and_broadcast(vec![execute.into_any()?], &default_gas(config)?)
        .await?;

    Ok(())
}

pub(crate) async fn axelar_source_gateway_verify_messages(
    message: &router_api::Message,
    cosmwasm_signer: &SigningClient,
    source_axelar_gateway: &cosmrs::AccountId,
    config: &AxelarConfiguration,
) -> eyre::Result<()> {
    tracing::info!(?message, "Axelar gateway.verify_messages");
    let msg = gateway_api::msg::ExecuteMsg::VerifyMessages(vec![message.clone()]);
    let execute = MsgExecuteContract {
        sender: cosmwasm_signer.signer_account_id()?,
        msg: serde_json::to_vec(&msg)?,
        funds: vec![],
        contract: source_axelar_gateway.clone(),
    };
    let _response = cosmwasm_signer
        .sign_and_broadcast(vec![execute.into_any()?], &default_gas(config)?)
        .await?;
    tracing::info!("sleeping for 30 seconds for the verifiers to respond");
    tokio::time::sleep(Duration::from_secs(30)).await;
    Ok(())
}
