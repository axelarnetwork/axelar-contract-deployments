use ethers::types::Address;
use evm_contracts_test_suite::evm_contracts_rs::contracts::{
    axelar_amplifier_gateway, axelar_memo,
};
use evm_contracts_test_suite::{ContractMiddleware, EvmSigner};

pub(crate) async fn deploy_axelar_memo(signer: EvmSigner, gateway: Address) -> eyre::Result<()> {
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

    Ok(())
}

pub(crate) async fn send_memo_to_solana(
    signer: EvmSigner,
    memo_contract: Address,
    memo_to_send: String,
    solana_chain_id: String,
) -> eyre::Result<()> {
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

    Ok(())
}
