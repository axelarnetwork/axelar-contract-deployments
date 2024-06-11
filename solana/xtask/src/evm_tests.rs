use clap::Parser;
use ethers::providers::Middleware;
use ethers::signers::Signer;
use ethers::types::{Address, U256};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_memo;
use evm_contracts_test_suite::ContractMiddleware;
use serial_test::serial;

use crate::cli::Cli;

#[test_log::test(tokio::test)]
async fn deploy_evm_axelar_memo_program() {
    // setup
    let evm_chain = evm_contracts_test_suite::chain::TestBlockchain::new();
    let alice = evm_chain.construct_provider_with_signer(0);
    let key = alice.wallet.signer().to_bytes();
    let key = hex::encode(key);
    let gateway = Address::random();
    let gateway_str = hex::encode(gateway.to_fixed_bytes());

    // action
    let node_rpc = evm_chain.anvil.endpoint();
    let args = vec![
        "xtask",
        "evm",
        "--node-rpc",
        node_rpc.as_str(),
        "--admin-private-key",
        key.as_str(),
        "deploy-axelar-memo",
        "--gateway-contract-address",
        gateway_str.as_str(),
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    cli.run().await.unwrap();

    // assert
    // the address is deterministic because the evm chain is always with a fresh
    // state
    let expected_deployment_address =
        hex::decode("da2686cef039093975589bb41cbae0f4767c3086").unwrap();
    let expected_deployment_address = Address::from_slice(&expected_deployment_address);
    let axelar_memo_contract = axelar_memo::AxelarMemo::<ContractMiddleware>::new(
        expected_deployment_address,
        alice.signer.clone(),
    );
    let received_messages = axelar_memo_contract
        .messages_received()
        .call()
        .await
        .expect("cannot communicate with the contract. Is it depoyed at the expected address?");
    assert_eq!(received_messages, U256::zero());
}

#[test_log::test(tokio::test)]
async fn send_memo_to_axelar_memo_program() {
    // setup
    let evm_chain = evm_contracts_test_suite::chain::TestBlockchain::new();
    let alice = evm_chain.construct_provider_with_signer(0);
    let bob = evm_chain.construct_provider_with_signer(1);
    let gateway = alice
        .deploy_axelar_amplifier_gateway(&[], alice.wallet.address(), alice.wallet.address())
        .await
        .unwrap();
    let axelar_memo_deployment = alice.deploy_axelar_memo(gateway).await.unwrap();

    // action
    let key = bob.wallet.signer().to_bytes();
    let key = hex::encode(key);
    let axelar_memo_deployment = hex::encode(axelar_memo_deployment.address().to_fixed_bytes());
    let node_rpc = evm_chain.anvil.endpoint();
    let args = vec![
        "xtask",
        "evm",
        "--node-rpc",
        node_rpc.as_str(),
        "--admin-private-key",
        key.as_str(),
        "send-memo-to-solana",
        "--evm-memo-contract-address",
        axelar_memo_deployment.as_str(),
        "--memo-to-send",
        "✅✅✅",
        "--solana-chain-id",
        "my-solana-chain-id",
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    cli.run().await.unwrap();

    // assert
    let tx_count = evm_chain
        .provider
        .get_transaction_count(bob.wallet.address(), None)
        .await
        .unwrap();
    assert_eq!(tx_count, U256::one());
}
