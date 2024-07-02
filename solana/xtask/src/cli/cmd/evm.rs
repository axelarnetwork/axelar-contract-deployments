use ethers::types::{Address, TransactionReceipt};
use evm_contracts_test_suite::evm_contracts_rs::contracts::{
    axelar_amplifier_gateway, axelar_memo,
};
use evm_contracts_test_suite::{ContractMiddleware, EvmSigner};

#[tracing::instrument]
pub(crate) async fn deploy_axelar_memo(
    signer: EvmSigner,
    gateway: Address,
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

    Ok(contract.address())
}

#[tracing::instrument]
pub(crate) async fn send_memo_to_solana(
    signer: EvmSigner,
    memo_contract: Address,
    memo_to_send: &str,
    solana_chain_id: &str,
) -> eyre::Result<TransactionReceipt> {
    tracing::info!(addr = ?signer.signer.address(), "sending memo");
    let memo_contract =
        axelar_memo::AxelarMemo::<ContractMiddleware>::new(memo_contract, signer.signer.clone());
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
            solana_chain_id.as_bytes().to_vec().into(),
            memo_to_send.as_bytes().to_vec().into(),
            vec![counter_account],
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    tracing::info!(tx_hash =? receipt.transaction_hash, "memo sent to the EVM Gateway");

    Ok(receipt)
}

#[tracing::instrument]
pub(crate) async fn send_memo_from_evm_to_evm(
    signer: EvmSigner,
    memo_contract: Address,
    memo_to_send: String,
    chain_id: String,
    destination_contract: String,
) -> eyre::Result<TransactionReceipt> {
    tracing::info!(addr = ?signer.signer.address(), "sending memo");
    let memo_contract =
        axelar_memo::AxelarMemo::<ContractMiddleware>::new(memo_contract, signer.signer.clone());
    let receipt = memo_contract
        .send_to_evm(
            destination_contract,
            chain_id.as_bytes().to_vec().into(),
            ethers::types::Bytes::from_iter(memo_to_send.as_bytes()),
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    tracing::info!(tx_hash =? receipt.transaction_hash, "memo sent to the EVM Gateway");

    Ok(receipt)
}
