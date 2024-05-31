use ethers::types::{Address, U256};
use ethers::utils::keccak256;
use evm_contracts_rs::contracts::{
    axelar_amplifier_gateway, axelar_amplifier_gateway_proxy, axelar_memo, example_encoder,
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
    pub async fn deploy_axelar_memo(
        &self,
        gateway: axelar_amplifier_gateway::AxelarAmplifierGateway<ContractMiddleware>,
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
