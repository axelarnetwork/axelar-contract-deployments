use evm_contracts_rs::contracts::{
    axelar_auth_weighted, axelar_gateway, axelar_memo, example_encoder,
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
    pub async fn deploy_axelar_gateway(
        &self,
        auth_weighted: &axelar_auth_weighted::AxelarAuthWeighted<ContractMiddleware>,
    ) -> anyhow::Result<axelar_gateway::AxelarGateway<ContractMiddleware>> {
        let contract =
            axelar_gateway::AxelarGateway::deploy(self.signer.clone(), auth_weighted.address())?
                .send()
                .await?;
        Ok(axelar_gateway::AxelarGateway::<ContractMiddleware>::new(
            contract.address(),
            self.signer.clone(),
        ))
    }

    /// Deploys the `AxelarAuthWeighted` contract.
    pub async fn deploy_axelar_auth_weighted(
        &self,
        recent_signer_sets: &[crate::evm_operators::OperatorSet],
    ) -> anyhow::Result<axelar_auth_weighted::AxelarAuthWeighted<ContractMiddleware>> {
        let constructor_params =
            crate::evm_operators::get_weighted_auth_deploy_param(recent_signer_sets);

        let contract = axelar_auth_weighted::AxelarAuthWeighted::deploy(
            self.signer.clone(),
            constructor_params,
        )?
        .send()
        .await?;
        Ok(
            axelar_auth_weighted::AxelarAuthWeighted::<ContractMiddleware>::new(
                contract.address(),
                self.signer.clone(),
            ),
        )
    }

    /// Deploys the `AxelarMemo` contract.
    pub async fn deploy_axelar_memo(
        &self,
        gateway: axelar_gateway::AxelarGateway<ContractMiddleware>,
    ) -> anyhow::Result<axelar_memo::AxelarMemo<ContractMiddleware>> {
        let contract = axelar_memo::AxelarMemo::deploy(self.signer.clone(), gateway.address())?
            .send()
            .await?;
        Ok(axelar_memo::AxelarMemo::<ContractMiddleware>::new(
            contract.address(),
            self.signer.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;
    use test_log::test;

    use crate::chain::TestBlockchain;
    use crate::evm_operators::create_operator_set;

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
    async fn can_deploy_axelar_auth_weighted() {
        // Setup
        let chain = TestBlockchain::new();
        let alice = chain.construct_provider_with_signer(0);
        let operators1 = create_operator_set(&chain, 0..5);
        let operators2 = create_operator_set(&chain, 5..10);

        // Action
        let _contract = alice
            .deploy_axelar_auth_weighted(&[operators1, operators2])
            .await
            .unwrap();
    }

    #[rstest]
    #[timeout(std::time::Duration::from_secs(2))]
    #[test(tokio::test)]
    async fn can_deploy_axelar_gateway() {
        // Setup
        let chain = TestBlockchain::new();
        let alice = chain.construct_provider_with_signer(0);
        let aw = alice.deploy_axelar_auth_weighted(&[]).await.unwrap();

        // Action
        let _contract = alice.deploy_axelar_gateway(&aw).await.unwrap();
    }

    #[rstest]
    #[timeout(std::time::Duration::from_secs(2))]
    #[test(tokio::test)]
    async fn can_deploy_axelar_memo() {
        // Setup
        let chain = TestBlockchain::new();
        let alice = chain.construct_provider_with_signer(0);
        let aw = alice.deploy_axelar_auth_weighted(&[]).await.unwrap();
        let gateway = alice.deploy_axelar_gateway(&aw).await.unwrap();

        // Action
        let _contract = alice.deploy_axelar_memo(gateway).await.unwrap();
    }
}
