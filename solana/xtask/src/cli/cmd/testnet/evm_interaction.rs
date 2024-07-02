use std::str::FromStr;
use std::time::Duration;

use ethers::abi::RawLog;
use ethers::contract::EthEvent;
use ethers::providers::Middleware;
use ethers::types::{Address as EvmAddress, TransactionRequest};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_memo;
use evm_contracts_test_suite::{ContractMiddleware, EvmSigner};
use router_api::{Address, ChainName, CrossChainId};

use super::devnet_amplifier::EvmChain;

#[tracing::instrument(skip_all, ret)]
pub(crate) fn create_axelar_message_from_evm_log(
    tx: &ethers::types::TransactionReceipt,
    source_chain: &EvmChain,
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

pub(crate) async fn call_execute_on_destination_evm_contract(
    message: router_api::Message,
    destination_memo_contract: ethers::types::H160,
    destination_evm_signer: EvmSigner,
    payload: ethers::types::Bytes,
) -> eyre::Result<()> {
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

pub(crate) async fn approve_messages_on_evm_gateway(
    destination_chain: &EvmChain,
    execute_data: String,
    destination_evm_signer: &EvmSigner,
) -> eyre::Result<()> {
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
