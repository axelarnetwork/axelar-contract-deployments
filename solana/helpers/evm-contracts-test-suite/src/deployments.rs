use anyhow::anyhow;
use ethers::contract::ContractFactory;
use ethers::providers::Middleware;
use ethers::signers::Signer;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Address, U256};
use ethers::utils::keccak256;
use evm_contracts_rs::contracts::{
    axelar_amplifier_gateway, axelar_amplifier_gateway_proxy, axelar_memo, axelar_solana_multicall,
    example_encoder,
};

use crate::ContractMiddleware;

impl crate::EvmSigner {
    /// Deploys the `ExampleEncoder` contract.
    ///
    /// This function deploys the `ExampleEncoder` contract using the signer
    /// stored in the `EvmSigner` instance. The deployment is done on the
    /// blockchain network that the signer is connected to.
    pub async fn deploy_example_encoder(
        &self,
    ) -> anyhow::Result<example_encoder::ExampleEncoder<ContractMiddleware>> {
        let contract = example_encoder::ExampleEncoder::deploy(self.signer.clone(), ())?
            .send()
            .await?;
        Ok(example_encoder::ExampleEncoder::new(
            contract.address(),
            self.signer.clone(),
        ))
    }

    /// Deploys the `AxelarGateway` contract.
    ///
    /// This function deploys the `AxelarGateway` contract using the signer
    /// stored in the `EvmSigner` instance. The deployment is done on the
    /// blockchain network that the signer is connected to.
    pub async fn deploy_axelar_amplifier_gateway(
        &self,
        recent_signer_sets: &[crate::evm_weighted_signers::WeightedSigners],
        owner: Address,
        operator: Address,
    ) -> anyhow::Result<axelar_amplifier_gateway::AxelarAmplifierGateway<ContractMiddleware>> {
        let previous_signer_retention = U256::from(4_u128);
        let minimum_rotation_delay = U256::from(1_u128);
        let domain_separator = get_domain_separator();
        let contract = axelar_amplifier_gateway::AxelarAmplifierGateway::deploy(
            self.signer.clone(),
            (
                previous_signer_retention,
                domain_separator,
                minimum_rotation_delay,
            ),
        )?
        .send()
        .await?;

        let bytes_params = crate::evm_weighted_signers::get_gateway_proxy_setup_signers(
            recent_signer_sets,
            operator,
        );
        let proxy = axelar_amplifier_gateway_proxy::AxelarAmplifierGatewayProxy::deploy(
            self.signer.clone(),
            (
                contract.address(),
                owner,
                ethers::abi::Token::Bytes(bytes_params),
            ),
        )?
        .send()
        .await?;
        Ok(axelar_amplifier_gateway::AxelarAmplifierGateway::<
            ContractMiddleware,
        >::new(proxy.address(), self.signer.clone()))
    }

    /// Deploys the `AxelarMemo` contract.
    #[tracing::instrument(skip_all, err)]
    pub async fn deploy_axelar_memo(
        &self,
        gateway: axelar_amplifier_gateway::AxelarAmplifierGateway<ContractMiddleware>,
    ) -> anyhow::Result<axelar_memo::AxelarMemo<ContractMiddleware>> {
        let factory = ContractFactory::new(
            axelar_memo::AXELARMEMO_ABI.clone(),
            axelar_memo::AXELARMEMO_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy(gateway.address())?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(axelar_memo::AxelarMemo::<ContractMiddleware>::new(
            contract,
            self.signer.clone(),
        ))
    }

    /// Deploys the `AxelarSolanaMultiCall` contract.
    pub async fn deploy_solana_multicall(
        &self,
        gateway: axelar_amplifier_gateway::AxelarAmplifierGateway<ContractMiddleware>,
    ) -> anyhow::Result<axelar_solana_multicall::AxelarSolanaMultiCall<ContractMiddleware>> {
        let factory = ContractFactory::new(
            axelar_solana_multicall::AXELARSOLANAMULTICALL_ABI.clone(),
            axelar_solana_multicall::AXELARSOLANAMULTICALL_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy(gateway.address())?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(axelar_solana_multicall::AxelarSolanaMultiCall::<
            ContractMiddleware,
        >::new(contract, self.signer.clone()))
    }

    /// This is useful when deploying to evm networks like avalanche-fuji, where
    /// otherwise `ethers-rs` would show up an error like "transaction dropped
    /// from mempool"
    async fn deploy_custom_poll(
        &self,
        mut tx: TypedTransaction,
    ) -> Result<ethers::types::H160, anyhow::Error> {
        self.signer.fill_transaction(&mut tx, None).await?;
        let _signature = self.wallet.sign_transaction(&tx).await?;
        let res = self.signer.send_transaction(tx.clone(), None).await?;
        let res = await_receipt(res).await?;
        let contract = res
            .contract_address
            .ok_or(anyhow!("no contract address in the receipt"))?;
        Ok(contract)
    }
}

/// helper method to await for tx receipts on slow networks
pub async fn await_receipt(
    res: ethers::providers::PendingTransaction<'_, ethers::providers::Http>,
) -> anyhow::Result<ethers::types::TransactionReceipt> {
    let res = res
        .retries(10)
        .interval(std::time::Duration::from_millis(500))
        .log_msg("deployment")
        .log()
        .await?
        .ok_or(anyhow!("no tx receipt"))?;
    Ok(res)
}

/// Return a hardcoded domain separator
/// This is used to append to message hashes when signers are signing a payload
pub fn get_domain_separator() -> [u8; 32] {
    let chaint_name = "chain";
    let router = "router";

    keccak256(format!("{chaint_name}{router}axelar-1"))
}

#[cfg(test)]
mod tests {

    use rstest::rstest;
    use test_log::test;

    use crate::chain::TestBlockchain;
    use crate::evm_weighted_signers::create_operator_set;

    #[rstest]
    #[timeout(std::time::Duration::from_secs(2))]
    #[test(tokio::test)]
    async fn can_deploy_example_encoder() {
        // Setup
        let chain = TestBlockchain::new();
        let alice = chain.construct_provider_with_signer(0);

        // Action
        let _contract = alice.deploy_example_encoder().await.unwrap();
    }

    #[rstest]
    #[timeout(std::time::Duration::from_secs(2))]
    #[test(tokio::test)]
    async fn can_deploy_axelar_amplifier_gateway() {
        // Setup
        let chain = TestBlockchain::new();
        let alice = chain.construct_provider_with_signer(0);
        let bob = chain.construct_provider_with_signer(1);
        let operators1 = create_operator_set(&chain, 0..5);

        // Action
        let _contract = alice
            .deploy_axelar_amplifier_gateway(
                &[operators1],
                alice.signer.address(),
                bob.signer.address(),
            )
            .await
            .unwrap();
    }

    #[rstest]
    #[timeout(std::time::Duration::from_secs(2))]
    #[test(tokio::test)]
    async fn can_deploy_axelar_memo() {
        // Setup
        let chain = TestBlockchain::new();
        let alice = chain.construct_provider_with_signer(0);
        let bob = chain.construct_provider_with_signer(1);
        let operators1 = create_operator_set(&chain, 0..5);

        let gateway = alice
            .deploy_axelar_amplifier_gateway(
                &[operators1],
                alice.signer.address(),
                bob.signer.address(),
            )
            .await
            .unwrap();
        // Action
        let _contract = alice.deploy_axelar_memo(gateway).await.unwrap();
    }
}
