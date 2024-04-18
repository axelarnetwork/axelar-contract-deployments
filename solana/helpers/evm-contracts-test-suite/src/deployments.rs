use contracts::example_encoder;
use evm_contracts_rs::contracts;

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
        Ok(contracts::ExampleEncoder::new(
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

    #[rstest]
    #[timeout(std::time::Duration::from_secs(2))]
    #[test(tokio::test)]
    async fn can_deploy_example_encoder() {
        // Setup
        let chain = TestBlockchain::new();
        let alice = chain.construct_provider_with_signer(0);
        let contract = alice.deploy_example_encoder().await.unwrap();

        // Action
        let address = contract.address();
        dbg!(address);
    }
}
