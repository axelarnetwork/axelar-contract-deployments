use ethers::abi::AbiDecode;
use ethers::types::{Address, TransactionReceipt, H160};
use ethers::utils::hex::ToHexExt;
use evm_contracts_test_suite::evm_contracts_rs::contracts::{
    axelar_amplifier_gateway, axelar_memo,
};
use evm_contracts_test_suite::{ContractMiddleware, EvmSigner};
use eyre::OptionExt;

use super::deployments::CustomEvmChainDeployments;

#[tracing::instrument(skip(signer, our_evm_deployment_tracker), fields(signer = ?signer.signer.address().encode_hex_with_prefix()))]
pub(crate) async fn deploy_axelar_memo(
    signer: EvmSigner,
    gateway: Address,
    our_evm_deployment_tracker: &mut CustomEvmChainDeployments,
) -> eyre::Result<Address> {
    tracing::info!("about to deploy AxelarMemo program");
    let gateway = axelar_amplifier_gateway::AxelarAmplifierGateway::<ContractMiddleware>::new(
        gateway,
        signer.signer.clone(),
    );
    let contract = signer
        .deploy_axelar_memo(gateway)
        .await
        .map_err(|err| eyre::eyre!(err))?;
    tracing::info!(memo_program =? contract.address(), "EVM Axelar Memo deployed");

    our_evm_deployment_tracker.memo_program_address =
        Some(contract.address().encode_hex_with_prefix());

    Ok(contract.address())
}

#[tracing::instrument(skip(signer, our_evm_deployment_tracker), fields(signer = ?signer.signer.address().encode_hex_with_prefix()))]
pub(crate) async fn send_memo_to_solana(
    signer: EvmSigner,
    memo_to_send: &str,
    solana_chain_name_on_axelar: &str,
    our_evm_deployment_tracker: &CustomEvmChainDeployments,
) -> eyre::Result<TransactionReceipt> {
    let memo_contract_address = H160::decode_hex(
        our_evm_deployment_tracker
            .memo_program_address
            .as_ref()
            .ok_or_eyre("memo contract not deployed")?,
    )?;
    tracing::info!(addr = ?signer.signer.address(), memo_contract = ? memo_contract_address.encode_hex(),"sending memo");
    let memo_contract = axelar_memo::AxelarMemo::<ContractMiddleware>::new(
        memo_contract_address,
        signer.signer.clone(),
    );
    let gateway_root_pda = gmp_gateway::get_gateway_root_config_pda().0;
    let (counter_pda, _counter_bump) =
        axelar_solana_memo_program::get_counter_pda(&gateway_root_pda);
    let counter_account = axelar_memo::SolanaAccountRepr {
        pubkey: counter_pda.to_bytes(),
        is_signer: false,
        is_writable: true,
    };
    let receipt = memo_contract
        .send_to_solana(
            axelar_solana_memo_program::id().to_string(),
            solana_chain_name_on_axelar.as_bytes().to_vec().into(),
            memo_to_send.as_bytes().to_vec().into(),
            vec![counter_account],
        )
        .send()
        .await?
        .await?
        .ok_or_eyre("tx receipt not available")?;

    tracing::info!(tx_hash =? receipt.transaction_hash, "memo sent to the EVM Gateway");

    Ok(receipt)
}

#[tracing::instrument(skip(signer, our_source_evm_deployment_tracker, our_destination_evm_deployment_tracker), fields(signer = ?signer.signer.address().encode_hex_with_prefix()))]
pub(crate) async fn send_memo_from_evm_to_evm(
    signer: EvmSigner,
    memo_to_send: String,
    our_destination_evm_deployment_tracker: &CustomEvmChainDeployments,
    our_source_evm_deployment_tracker: &CustomEvmChainDeployments,
) -> eyre::Result<TransactionReceipt> {
    let our_memo_contract_address = H160::decode_hex(
        our_source_evm_deployment_tracker
            .memo_program_address
            .as_ref()
            .ok_or_eyre("memo contract not deployed")?,
    )?;

    let destination_memo_contract_address = our_source_evm_deployment_tracker
        .memo_program_address
        .as_ref()
        .ok_or_eyre("memo contract not deployed")?;
    tracing::info!(addr = ?signer.signer.address(), memo_contract = ? our_memo_contract_address.encode_hex(),"sending memo");
    let memo_contract = axelar_memo::AxelarMemo::<ContractMiddleware>::new(
        our_memo_contract_address,
        signer.signer.clone(),
    );
    let receipt = memo_contract
        .send_to_evm(
            destination_memo_contract_address.clone(),
            our_destination_evm_deployment_tracker
                .id
                .as_bytes()
                .to_vec()
                .into(),
            ethers::types::Bytes::from_iter(memo_to_send.as_bytes()),
        )
        .send()
        .await?
        .await?
        .ok_or_eyre("tx receipt not available")?;

    tracing::info!(tx_hash =? receipt.transaction_hash, "memo sent to the EVM Gateway");

    Ok(receipt)
}
