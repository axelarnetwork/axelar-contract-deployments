use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvmChain {
    pub(crate) name: String,
    pub(crate) id: String,
    pub(crate) axelar_id: String,
    pub(crate) chain_id: u64,
    pub(crate) rpc: String,
    pub(crate) token_symbol: String,
    pub(crate) confirmations: u64,
    pub(crate) contracts: Contracts,
    pub(crate) explorer: Explorer,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GasOptions {
    pub(crate) gas_limit: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Contracts {
    pub(crate) axelar_gateway: Option<AxelarGateway>,
    pub(crate) interchain_governance: Option<InterchainGovernance>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AxelarGateway {
    pub(crate) address: String,
    pub(crate) deployer: String,
    pub(crate) implementation: String,
    pub(crate) implementation_codehash: String,
    pub(crate) deployment_method: String,
    pub(crate) operator: String,
    pub(crate) previous_signers_retention: u64,
    pub(crate) domain_separator: String,
    pub(crate) minimum_rotation_delay: u64,
    pub(crate) salt: String,
    pub(crate) gas_options: GasOptions,
    pub(crate) proxy_deployment_args: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InterchainGovernance {
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConstAddressDeployer {
    pub(crate) address: String,
    pub(crate) deployer: String,
    pub(crate) deployment_method: String,
    pub(crate) codehash: String,
    pub(crate) predeploy_codehash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Create3Deployer {
    pub(crate) address: String,
    pub(crate) deployer: String,
    pub(crate) deployment_method: String,
    pub(crate) codehash: String,
    pub(crate) predeploy_codehash: String,
    pub(crate) salt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Explorer {
    #[serde(alias = "explorer")]
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) api: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AxelarChain {
    pub(crate) contracts: AxelarContracts,
    pub(crate) id: String,
    pub(crate) axelar_id: String,
    pub(crate) chain_id: String,
    pub(crate) rpc: String,
    pub(crate) lcd: String,
    pub(crate) grpc: String,
    pub(crate) token_symbol: String,
    pub(crate) gas_price: String,
    pub(crate) gas_limit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct AxelarContracts {
    pub(crate) service_registry: ServiceRegistry,
    pub(crate) router: Router,
    pub(crate) multisig: Multisig,
    pub(crate) coordinator: Coordinator,
    pub(crate) rewards: Rewards,
    pub(crate) nexus_gateway: NexusGateway,
    pub(crate) voting_verifier: MultichainAxelarDeployment<VotingVerifier>,
    pub(crate) gateway: MultichainAxelarDeployment<Gateway>,
    pub(crate) multisig_prover: MultichainAxelarDeployment<MultisigProver>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MultichainAxelarDeployment<T> {
    pub(crate) code_id: u64,
    #[serde(flatten)]
    pub(crate) networks: BTreeMap<String, T>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceRegistry {
    pub(crate) governance_account: String,
    pub(crate) code_id: u64,
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Router {
    pub(crate) admin_address: String,
    pub(crate) governance_address: String,
    pub(crate) code_id: u64,
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Multisig {
    pub(crate) governance_address: String,
    pub(crate) block_expiry: u64,
    pub(crate) code_id: u64,
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Coordinator {
    pub(crate) governance_address: String,
    pub(crate) code_id: u64,
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Rewards {
    pub(crate) governance_address: String,
    #[allow(clippy::struct_field_names)]
    pub(crate) rewards_denom: String,
    pub(crate) params: RewardParams,
    pub(crate) code_id: u64,
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RewardParams {
    pub(crate) epoch_duration: String,
    pub(crate) rewards_per_epoch: String,
    pub(crate) participation_threshold: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NexusGateway {
    pub(crate) nexus: String,
    pub(crate) code_id: u64,
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VotingVerifier {
    pub(crate) governance_address: String,
    pub(crate) service_name: String,
    pub(crate) source_gateway_address: String,
    pub(crate) voting_threshold: Vec<String>,
    pub(crate) block_expiry: u64,
    pub(crate) confirmation_height: u64,
    pub(crate) msg_id_format: String,
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Gateway {
    pub(crate) address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MultisigProver {
    pub(crate) governance_address: String,
    pub(crate) admin_address: String,
    #[serde(rename = "destinationChainID")]
    pub(crate) destination_chain_id: String,
    pub(crate) signing_threshold: Vec<String>,
    pub(crate) service_name: String,
    pub(crate) verifier_set_diff_threshold: u64,
    pub(crate) encoder: String,
    pub(crate) key_type: String,
    pub(crate) domain_separator: String,
    pub(crate) address: String,
}

// Root structure
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AxelarDeploymentRoot {
    pub(crate) chains: BTreeMap<String, EvmChain>,
    pub(crate) axelar: AxelarChain,
}

impl AxelarDeploymentRoot {
    pub(crate) fn from_reader(mut reader: impl std::io::Read) -> Self {
        let mut data = Vec::with_capacity(24_000); // the `devnet-amplifier.jsom` is ~22kb
        reader.read_to_end(&mut data).unwrap();

        let data = simd_json::from_slice::<AxelarDeploymentRoot>(data.as_mut_slice()).unwrap();
        data
    }

    pub(crate) fn get_evm_chain(&self, evm_chain: &str) -> eyre::Result<EvmChain> {
        let chain = self
            .chains
            .get(evm_chain)
            .ok_or_else(|| {
                let allowed_chains = self.chains.keys().collect::<Vec<_>>();
                eyre::eyre!("allowed chain values are {allowed_chains:?}")
            })?
            .clone();

        tracing::info!(?chain, "resolved evm chain");
        Ok(chain.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::cmd::path::xtask_crate_root_dir;

    #[test]
    fn can_deserialize_root() {
        let root = get_data();
        let mut data = simd_json::to_string(&root).unwrap().into_bytes();
        let _ = serde_json::from_slice::<AxelarDeploymentRoot>(data.as_mut_slice()).unwrap();
    }

    #[test]
    fn can_deserialize_evm_deployments() {
        let root = get_data();
        let mut data = simd_json::to_string(&root["chains"]).unwrap().into_bytes();
        let _ = simd_json::from_slice::<BTreeMap<String, EvmChain>>(data.as_mut_slice()).unwrap();
    }

    #[test]
    fn can_deserialize_cosmwasm_deployment() {
        let root = get_data();
        let mut data = simd_json::to_string(&root["axelar"]).unwrap().into_bytes();
        let _ = simd_json::from_slice::<AxelarChain>(data.as_mut_slice()).unwrap();
    }

    fn get_data() -> simd_json::owned::Value {
        let mut data = std::fs::read(xtask_crate_root_dir().join("devnet-amplifier.json")).unwrap();
        simd_json::to_owned_value(data.as_mut_slice()).unwrap()
    }
}
