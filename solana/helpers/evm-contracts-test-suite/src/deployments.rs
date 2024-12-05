use std::iter::once;

use anyhow::anyhow;
use ethers::contract::ContractFactory;
use ethers::providers::Middleware;
use ethers::signers::Signer;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Address, Bytes, U256};
use ethers::utils::keccak256;
use evm_contracts_rs::contracts::{
    axelar_amplifier_gateway, axelar_amplifier_gateway_proxy, axelar_auth_weighted,
    axelar_create3_deployer, axelar_gas_service, axelar_memo, axelar_solana_multicall,
    example_encoder, gateway_caller, interchain_proxy, interchain_token, interchain_token_deployer,
    interchain_token_factory, interchain_token_service, test_canonical_token, token_handler,
    token_manager, token_manager_deployer,
};

use crate::ContractMiddleware;

const INTERCHAIN_TOKEN_SERVICE_DEPLOYMENT_KEY: &str = "InterchainTokenService";
const INTERCHAIN_TOKEN_FACTORY_DEPLOYMENT_KEY: &str = "InterchainTokenFactory";
const CHAIN_NAME: &str = "chain";

/// Struct that holds all the ITS contracts
pub struct ItsContracts {
    /// InterchainTokenService contract
    pub interchain_token_service:
        interchain_token_service::InterchainTokenService<ContractMiddleware>,

    /// AxelarAuthWeighted contract
    pub auth_weighted: axelar_auth_weighted::AxelarAuthWeighted<ContractMiddleware>,

    /// AxelarGateway contract
    pub gateway: axelar_amplifier_gateway::AxelarAmplifierGateway<ContractMiddleware>,

    /// AxelarGasService contract
    pub gas_service: axelar_gas_service::AxelarGasService<ContractMiddleware>,

    /// InterchainTokenFactory contract
    pub interchain_token_factory:
        interchain_token_factory::InterchainTokenFactory<ContractMiddleware>,

    /// Create3Deploy contract
    pub create3_deployer: axelar_create3_deployer::Create3Deployer<ContractMiddleware>,

    /// TokenManagerDeployer contract
    pub token_manager_deployer: token_manager_deployer::TokenManagerDeployer<ContractMiddleware>,

    /// InterchainToken contract
    pub interchain_token: interchain_token::InterchainToken<ContractMiddleware>,

    /// InterchainTokenDeployer contract
    pub interchain_token_deployer:
        interchain_token_deployer::InterchainTokenDeployer<ContractMiddleware>,

    /// TokenManager contract
    pub token_manager: token_manager::TokenManager<ContractMiddleware>,

    /// GatewayCaller contract
    pub gateway_caller: gateway_caller::GatewayCaller<ContractMiddleware>,
}

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

    /// Deploys the `AxelarAuthWeighted` contract.
    pub async fn deploy_axelar_auth_weighted(
        &self,
        operator_sets: &[crate::evm_weighted_signers::WeightedSigners],
    ) -> anyhow::Result<axelar_auth_weighted::AxelarAuthWeighted<ContractMiddleware>> {
        let factory = ContractFactory::new(
            axelar_auth_weighted::AXELARAUTHWEIGHTED_ABI.clone(),
            axelar_auth_weighted::AXELARAUTHWEIGHTED_BYTECODE.clone(),
            self.signer.clone(),
        );
        let params = crate::evm_weighted_signers::get_weighted_auth_deploy_param(operator_sets);
        let deployer = factory.deploy(params)?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;

        Ok(
            axelar_auth_weighted::AxelarAuthWeighted::<ContractMiddleware>::new(
                contract,
                self.signer.clone(),
            ),
        )
    }

    /// Deploys the `AxelarGasService` contract.
    pub async fn deploy_axelar_gas_service(
        &self,
    ) -> anyhow::Result<axelar_gas_service::AxelarGasService<ContractMiddleware>> {
        let factory = ContractFactory::new(
            axelar_gas_service::AXELARGASSERVICE_ABI.clone(),
            axelar_gas_service::AXELARGASSERVICE_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy(self.wallet.address())?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(
            axelar_gas_service::AxelarGasService::<ContractMiddleware>::new(
                contract,
                self.signer.clone(),
            ),
        )
    }

    /// Deploys the `Create3Deployer` contract.
    pub async fn deploy_axelar_create3_deployer(
        &self,
    ) -> anyhow::Result<axelar_create3_deployer::Create3Deployer<ContractMiddleware>> {
        let factory = ContractFactory::new(
            axelar_create3_deployer::CREATE3DEPLOYER_ABI.clone(),
            axelar_create3_deployer::CREATE3DEPLOYER_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy(())?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(
            axelar_create3_deployer::Create3Deployer::<ContractMiddleware>::new(
                contract,
                self.signer.clone(),
            ),
        )
    }

    /// Deploys the `TokenManagerDeployer` contract.
    pub async fn deploy_token_manager_deployer(
        &self,
    ) -> anyhow::Result<token_manager_deployer::TokenManagerDeployer<ContractMiddleware>> {
        let factory = ContractFactory::new(
            token_manager_deployer::TOKENMANAGERDEPLOYER_ABI.clone(),
            token_manager_deployer::TOKENMANAGERDEPLOYER_BYTECODE.clone(),
            self.signer.clone(),
        );

        let deployer = factory.deploy(())?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(token_manager_deployer::TokenManagerDeployer::<
            ContractMiddleware,
        >::new(contract, self.signer.clone()))
    }

    /// Deploys the `InterchainToken` contract.
    pub async fn deploy_interchain_token(
        &self,
        interchain_token_service_address: Address,
    ) -> anyhow::Result<interchain_token::InterchainToken<ContractMiddleware>> {
        let factory = ContractFactory::new(
            interchain_token::INTERCHAINTOKEN_ABI.clone(),
            interchain_token::INTERCHAINTOKEN_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy(interchain_token_service_address)?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(
            interchain_token::InterchainToken::<ContractMiddleware>::new(
                contract,
                self.signer.clone(),
            ),
        )
    }

    /// Deploys the `InterchainTokenDeployer` contract.
    pub async fn deploy_interchain_token_deployer(
        &self,
        interchain_token_address: Address,
    ) -> anyhow::Result<interchain_token_deployer::InterchainTokenDeployer<ContractMiddleware>>
    {
        let factory = ContractFactory::new(
            interchain_token_deployer::INTERCHAINTOKENDEPLOYER_ABI.clone(),
            interchain_token_deployer::INTERCHAINTOKENDEPLOYER_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy(interchain_token_address)?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(interchain_token_deployer::InterchainTokenDeployer::<
            ContractMiddleware,
        >::new(contract, self.signer.clone()))
    }

    /// Deploys the `TokenManager` contract.
    pub async fn deploy_token_manager(
        &self,
        interchain_token_service_address: Address,
    ) -> anyhow::Result<token_manager::TokenManager<ContractMiddleware>> {
        let factory = ContractFactory::new(
            token_manager::TOKENMANAGER_ABI.clone(),
            token_manager::TOKENMANAGER_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy(interchain_token_service_address)?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(token_manager::TokenManager::<ContractMiddleware>::new(
            contract,
            self.signer.clone(),
        ))
    }

    /// Deploys the `TokenHandler` contract.
    pub async fn deploy_token_handler(
        &self,
        gateway: Address,
    ) -> anyhow::Result<token_handler::TokenHandler<ContractMiddleware>> {
        let factory = ContractFactory::new(
            token_handler::TOKENHANDLER_ABI.clone(),
            token_handler::TOKENHANDLER_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy(gateway)?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(token_handler::TokenHandler::<ContractMiddleware>::new(
            contract,
            self.signer.clone(),
        ))
    }

    /// Deploys the `GatewayCaller` contract.
    pub async fn deploy_gateway_caller(
        &self,
        gateway: Address,
        gas_service: Address,
    ) -> anyhow::Result<gateway_caller::GatewayCaller<ContractMiddleware>> {
        let factory = ContractFactory::new(
            gateway_caller::GATEWAYCALLER_ABI.clone(),
            gateway_caller::GATEWAYCALLER_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy((gateway, gas_service))?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(gateway_caller::GatewayCaller::<ContractMiddleware>::new(
            contract,
            self.signer.clone(),
        ))
    }

    /// Deploys the `InterchainTokenService` contract.
    #[allow(clippy::too_many_arguments)]
    pub async fn deploy_axelar_interchain_token_service(
        &self,
        create3_deployer: axelar_create3_deployer::Create3Deployer<ContractMiddleware>,
        token_manager_deployer_address: Address,
        interchain_token_deployer_address: Address,
        gateway_address: Address,
        gas_service: Address,
        interchain_token_factory_address: Address,
        token_manager_address: Address,
        token_handler_address: Address,
        gateway_caller_address: Address,
        chains: impl IntoIterator<Item = String>,
        owner: Address,
        operator: Address,
    ) -> anyhow::Result<interchain_token_service::InterchainTokenService<ContractMiddleware>> {
        let contract = interchain_token_service::InterchainTokenService::deploy(
            self.signer.clone(),
            (
                token_manager_deployer_address,
                interchain_token_deployer_address,
                gateway_address,
                gas_service,
                interchain_token_factory_address,
                CHAIN_NAME.to_string(),
                token_manager_address,
                token_handler_address,
                gateway_caller_address,
            ),
        )?
        .send()
        .await?;

        let factory = ContractFactory::new(
            interchain_proxy::INTERCHAINPROXY_ABI.clone(),
            interchain_proxy::INTERCHAINPROXY_BYTECODE.clone(),
            self.signer.clone(),
        );

        let salt = keccak256(ethers::abi::AbiEncode::encode(
            INTERCHAIN_TOKEN_SERVICE_DEPLOYMENT_KEY.to_string(),
        ));
        let its_address = create3_deployer
            .deployed_address(Bytes::new(), self.signer.address(), salt)
            .call()
            .await?;

        let (chains, addresses) = once(CHAIN_NAME.to_string())
            .chain(chains.into_iter())
            .map(|chain| {
                (
                    ethers::abi::Token::String(chain.to_string()),
                    ethers::abi::Token::String(its_address.to_string()),
                )
            })
            .unzip();

        let setup_args: Bytes = ethers::abi::encode(&[
            ethers::abi::Token::Address(operator),
            ethers::abi::Token::String(CHAIN_NAME.to_string()),
            ethers::abi::Token::Array(chains),
            ethers::abi::Token::Array(addresses),
        ])
        .into();

        let deployer = factory.deploy((contract.address(), owner, setup_args))?;
        let _receipt = create3_deployer
            .custom_deploy(deployer.tx.data().unwrap().clone(), salt)
            .send()
            .await?
            .await?;

        Ok(interchain_token_service::InterchainTokenService::<
            ContractMiddleware,
        >::new(its_address, self.signer.clone()))
    }

    /// Deploys the `InterchainTokenFactory` contract.
    pub async fn deploy_axelar_interchain_token_factory(
        &self,
        interchain_token_service_address: Address,
        create3_deployer: axelar_create3_deployer::Create3Deployer<ContractMiddleware>,
    ) -> anyhow::Result<interchain_token_factory::InterchainTokenFactory<ContractMiddleware>> {
        let contract = interchain_token_factory::InterchainTokenFactory::deploy(
            self.signer.clone(),
            interchain_token_service_address,
        )?
        .send()
        .await?;

        let factory = ContractFactory::new(
            interchain_proxy::INTERCHAINPROXY_ABI.clone(),
            interchain_proxy::INTERCHAINPROXY_BYTECODE.clone(),
            self.signer.clone(),
        );

        let deployer = factory.deploy((contract.address(), self.signer.address(), Bytes::new()))?;
        let salt = keccak256(ethers::abi::AbiEncode::encode(
            INTERCHAIN_TOKEN_FACTORY_DEPLOYMENT_KEY.to_string(),
        ));

        let factory_address = create3_deployer
            .deployed_address(Bytes::new(), self.signer.address(), salt)
            .call()
            .await?;

        let _receipt = create3_deployer
            .custom_deploy(deployer.tx.data().unwrap().clone(), salt)
            .send()
            .await?
            .await?;

        Ok(interchain_token_factory::InterchainTokenFactory::<
            ContractMiddleware,
        >::new(factory_address, self.signer.clone()))
    }

    /// Deploys the `TestCanonicalToken` contract.
    pub async fn deploy_axelar_test_canonical_token(
        &self,
        name: String,
        symbol: String,
        decimals: u8,
    ) -> anyhow::Result<test_canonical_token::TestCanonicalToken<ContractMiddleware>> {
        let factory = ContractFactory::new(
            test_canonical_token::TESTCANONICALTOKEN_ABI.clone(),
            test_canonical_token::TESTCANONICALTOKEN_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy((name, symbol, decimals))?;
        let contract = self.deploy_custom_poll(deployer.tx).await?;
        Ok(
            test_canonical_token::TestCanonicalToken::<ContractMiddleware>::new(
                contract,
                self.signer.clone(),
            ),
        )
    }

    /// Deploys all ITS contracts.
    pub async fn deploy_all_its(
        &self,
        owner: Address,
        operator: Address,
        operator_sets: &[crate::evm_weighted_signers::WeightedSigners],
        chains: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<ItsContracts> {
        let create3_deployer = self.deploy_axelar_create3_deployer().await?;
        let gas_service = self.deploy_axelar_gas_service().await?;
        let auth_weighted = self.deploy_axelar_auth_weighted(operator_sets).await?;

        let its_salt = keccak256(ethers::abi::AbiEncode::encode(
            INTERCHAIN_TOKEN_SERVICE_DEPLOYMENT_KEY.to_string(),
        ));
        let factory_salt = keccak256(ethers::abi::AbiEncode::encode(
            INTERCHAIN_TOKEN_FACTORY_DEPLOYMENT_KEY,
        ));

        let interchain_token_service_address = create3_deployer
            .deployed_address(Bytes::new(), self.signer.address(), its_salt)
            .call()
            .await?;
        let token_manager_deployer = self.deploy_token_manager_deployer().await?;
        let gateway = self
            .deploy_axelar_amplifier_gateway(operator_sets, owner, operator)
            .await?;
        let interchain_token = self
            .deploy_interchain_token(interchain_token_service_address)
            .await?;
        let interchain_token_deployer = self
            .deploy_interchain_token_deployer(interchain_token.address())
            .await?;
        let token_manager = self
            .deploy_token_manager(interchain_token_service_address)
            .await?;
        let token_handler = self.deploy_token_handler(gateway.address()).await?;
        let gateway_caller = self
            .deploy_gateway_caller(gateway.address(), gas_service.address())
            .await?;

        let interchain_token_factory_address = create3_deployer
            .deployed_address(Bytes::new(), self.signer.address(), factory_salt)
            .call()
            .await?;

        let interchain_token_service = self
            .deploy_axelar_interchain_token_service(
                create3_deployer.clone(),
                token_manager_deployer.address(),
                interchain_token_deployer.address(),
                gateway.address(),
                gas_service.address(),
                interchain_token_factory_address,
                token_manager.address(),
                token_handler.address(),
                gateway_caller.address(),
                chains,
                owner,
                operator,
            )
            .await?;

        let interchain_token_factory = self
            .deploy_axelar_interchain_token_factory(
                interchain_token_service_address,
                create3_deployer.clone(),
            )
            .await?;

        Ok(ItsContracts {
            interchain_token_service,
            auth_weighted,
            gateway,
            gas_service,
            interchain_token_factory,
            create3_deployer,
            token_manager_deployer,
            interchain_token,
            interchain_token_deployer,
            token_manager,
            gateway_caller,
        })
    }

    /// Deploys the `AxelarMemo` contract.
    #[tracing::instrument(skip_all, err)]
    pub async fn deploy_axelar_memo(
        &self,
        gateway: axelar_amplifier_gateway::AxelarAmplifierGateway<ContractMiddleware>,
        its: Option<interchain_token_service::InterchainTokenService<ContractMiddleware>>,
    ) -> anyhow::Result<axelar_memo::AxelarMemo<ContractMiddleware>> {
        let factory = ContractFactory::new(
            axelar_memo::AXELARMEMO_ABI.clone(),
            axelar_memo::AXELARMEMO_BYTECODE.clone(),
            self.signer.clone(),
        );
        let deployer = factory.deploy((
            gateway.address(),
            its.map(|c| c.address()).unwrap_or(Address::zero()),
        ))?;
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
    let router = "router";

    keccak256(format!("{CHAIN_NAME}{router}axelar-1"))
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
        let _contract = alice.deploy_axelar_memo(gateway, None).await.unwrap();
    }
}
