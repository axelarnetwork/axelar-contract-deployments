use std::str::FromStr;
use std::time::Duration;

use axelar_wasm_std::nonempty::Uint64;
use cosmrs::cosmwasm::MsgExecuteContract;
use cosmrs::tx::Msg;
use ethers::abi::RawLog;
use ethers::contract::EthEvent;
use ethers::providers::Middleware;
use ethers::types::{Address as EvmAddress, TransactionRequest};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_memo;
use evm_contracts_test_suite::{ContractMiddleware, EvmSigner};
use router_api::{Address, ChainName, CrossChainId};

use self::devnet_amplifier::EvmChain;
use super::cosmwasm::cosmos_client::signer::SigningClient;
use crate::cli::cmd::cosmwasm::{default_gas, ResponseEventExtract};
use crate::cli::cmd::evm::send_memo_to_evm;

#[allow(clippy::all, warnings, clippy::unreadable_literal)]
pub(crate) mod devnet_amplifier {
    include!(concat!(env!("OUT_DIR"), "/devnet_amplifier.rs"));
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
    let soruce_axelar_gateway = axelar_cosmwasm
        .gateway
        .get(source_chain.id.as_str())
        .unwrap();
    let destination_multisig_prover = axelar_cosmwasm
        .multisig_prover
        .get(destination_chain.id.as_str())
        .unwrap();

    let tx = send_memo_to_evm(
        source_evm_signer,
        source_memo_contract,
        memo_to_send.clone(),
        destination_chain.id.clone(),
        format!(
            "0x{}",
            hex::encode(destination_memo_contract.to_fixed_bytes())
        ),
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
    let (payload, message) = create_axelar_message_from_evm_log(&tx, source_chain);

    // verify messages on the Axelar cosmwasm gateway.verify_messages()
    axelar_source_gateway_verify_messages(&message, &cosmwasm_signer, soruce_axelar_gateway)
        .await?;

    check_voting_verifier_status(&message, &cosmwasm_signer, &axelar_cosmwasm, source_chain)
        .await?;

    // call route_messages()
    axelar_source_gateway_route_messages(&message, &cosmwasm_signer, soruce_axelar_gateway).await?;

    // construct_proof() at the multisig-prover corresponding to the destination
    // chain
    let execute_data = axelar_destination_multisig_prover_construct_proof(
        &message,
        destination_multisig_prover,
        cosmwasm_signer,
    )
    .await?;

    // Call the destination chain Gateway
    approve_messages_on_evm_gateway(destination_chain, execute_data, &destination_evm_signer)
        .await?;
    call_execute_on_destination_evm_contract(
        message,
        destination_memo_contract,
        destination_evm_signer,
        payload,
    )
    .await?;

    Ok(())
}

async fn check_voting_verifier_status(
    message: &router_api::Message,
    cosmwasm_signer: &SigningClient,
    axelar_cosmwasm: &devnet_amplifier::Axelar,
    source_chain: &EvmChain,
) -> Result<(), eyre::Error> {
    let vv_msg = voting_verifier::msg::QueryMsg::GetMessagesStatus {
        messages: vec![message.clone()],
    };
    let res = cosmwasm_signer
        .query::<serde_json::Value>(
            cosmrs::AccountId::from_str(
                axelar_cosmwasm
                    .voting_verifier
                    .get(source_chain.id.as_str())
                    .unwrap()
                    .address
                    .as_str(),
            )
            .unwrap(),
            serde_json::to_vec(&vv_msg).unwrap(),
        )
        .await?;
    tracing::info!(?res, "voting verifier status");
    Ok(())
}

async fn axelar_destination_multisig_prover_construct_proof(
    message: &router_api::Message,
    destination_multisig_prover: &devnet_amplifier::MultisigProver,
    cosmwasm_signer: SigningClient,
) -> Result<String, eyre::Error> {
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

async fn axelar_source_gateway_route_messages(
    message: &router_api::Message,
    cosmwasm_signer: &SigningClient,
    soruce_axelar_gateway: &devnet_amplifier::Contract,
) -> Result<(), eyre::Error> {
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

async fn axelar_source_gateway_verify_messages(
    message: &router_api::Message,
    cosmwasm_signer: &SigningClient,
    soruce_axelar_gateway: &devnet_amplifier::Contract,
) -> Result<(), eyre::Error> {
    tracing::info!("Axelar gateway.verify_messages");
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

#[tracing::instrument(skip_all, ret)]
fn create_axelar_message_from_evm_log(
    tx: &ethers::types::TransactionReceipt,
    source_chain: &devnet_amplifier::EvmChain,
) -> (ethers::types::Bytes, router_api::Message) {
    let log_index = 0;
    let log: RawLog = tx.logs.get(log_index).unwrap().clone().into();
    let log: ContractCallFilter = ContractCallFilter::decode_log(&log).unwrap();
    let payload = log.payload.clone();
    tracing::info!(?log, "evm memo log decoded");

    let encoded_id = &hex::encode(tx.transaction_hash.to_fixed_bytes());
    let message = router_api::Message {
        cc_id: CrossChainId {
            chain: ChainName::from_str(source_chain.name.as_str()).unwrap(),
            id: format!("0x{encoded_id}-{log_index}").parse().unwrap(),
        },
        source_address: Address::from_str(
            format!("0x{}", hex::encode(log.sender.to_fixed_bytes())).as_str(),
        )
        .unwrap(),
        destination_chain: ChainName::from_str(log.destination_chain.as_str()).unwrap(),
        destination_address: Address::from_str(log.destination_contract_address.as_str()).unwrap(),
        payload_hash: log.payload_hash,
    };
    (payload, message)
}

async fn call_execute_on_destination_evm_contract(
    message: router_api::Message,
    destination_memo_contract: ethers::types::H160,
    destination_evm_signer: EvmSigner,
    payload: ethers::types::Bytes,
) -> Result<(), eyre::Error> {
    let memo_contract = axelar_memo::AxelarMemo::<ContractMiddleware>::new(
        destination_memo_contract,
        destination_evm_signer.signer.clone(),
    );

    let source_chain = message.cc_id.chain.to_string();
    let message_id = message.cc_id.id.clone().to_string();
    let source_address = message.source_address.to_string();
    tracing::info!(
        source_chain,
        message_id,
        source_address,
        ?payload,
        "sending `execute` to the destination contract"
    );
    let _tx = memo_contract
        .execute(source_chain, message_id, source_address, payload)
        .send()
        .await?
        .await?
        .unwrap();
    Ok(())
}

async fn approve_messages_on_evm_gateway(
    destination_chain: &EvmChain,
    execute_data: String,
    destination_evm_signer: &EvmSigner,
) -> Result<(), eyre::Error> {
    let destination_evm_gateway = EvmAddress::from_slice(
        hex::decode(destination_chain.axelar_gateway.strip_prefix("0x").unwrap())
            .unwrap()
            .as_ref(),
    );
    let tx = TransactionRequest::new()
        .to(destination_evm_gateway)
        .data(hex::decode(execute_data).unwrap());
    tracing::info!("sending `approve_messages` tx to the destination gateway");
    let gateway_approve_msgs = destination_evm_signer
        .signer
        .send_transaction(tx, None)
        .await?
        .await?
        .unwrap();
    tracing::info!(tx =? gateway_approve_msgs, "success");
    tracing::info!("sleeping for 30 seconds for the change to settle");
    tokio::time::sleep(Duration::from_secs(30)).await;
    Ok(())
}

pub(crate) mod multisig_prover_api {
    // NOTE: there are issues with using `multisig-prover` as a dependency (bulid
    // breaks). Thats why the types are re-defined here
    use axelar_wasm_std::nonempty::Uint64;
    use cosmwasm_schema::cw_serde;
    use router_api::{CrossChainId, Message};

    #[cw_serde]
    pub(crate) enum MultisigProverExecuteMsg {
        ConstructProof { message_ids: Vec<CrossChainId> },
    }

    #[cw_serde]
    pub(crate) enum QueryMsg {
        GetProof { multisig_session_id: Uint64 },
    }

    #[cw_serde]
    pub(crate) struct GetProofResponse {
        pub(crate) multisig_session_id: Uint64,
        pub(crate) message_ids: Vec<CrossChainId>,
        pub(crate) payload: Payload,
        pub(crate) status: ProofStatus,
    }

    #[cw_serde]
    pub(crate) enum Payload {
        Messages(Vec<Message>),
    }

    #[cw_serde]
    pub(crate) enum ProofStatus {
        Pending,
        Completed { execute_data: String },
    }
}
