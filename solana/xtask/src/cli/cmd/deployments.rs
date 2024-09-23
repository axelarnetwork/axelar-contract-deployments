use std::collections::BTreeMap;
use std::fs::OpenOptions;

use axelar_wasm_std::msg_id::MessageIdFormat;
use multisig::key::KeyType;
use multisig::verifier_set::VerifierSet;
use solana_sdk::pubkey::Pubkey;

use super::axelar_deployments::{AxelarChain, Contracts, Explorer};
use super::cosmwasm::domain_separator;
use super::path::xtask_crate_root_dir;
use super::testnet::multisig_prover_api;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SolanaDeploymentRoot {
    pub(crate) solana_configuration: SolanaConfiguration,
    pub(crate) axelar_configuration: AxelarConfiguration,
    pub(crate) voting_verifier: Option<VotingVerifierDeployment>,
    pub(crate) axelar_gateway: Option<AxelarGatewayDeployment>,
    pub(crate) multisig_prover: Option<MultisigProverDeployment>,
    pub(crate) solana_gateway: Option<SolanaGatewayDeployment>,
    pub(crate) solana_memo_program: Option<SolanaMemoProgram>,
    pub(crate) evm_deployments: EvmDeployments,
}

impl SolanaDeploymentRoot {
    pub(crate) fn new(
        solana_chain_name_on_axelar_chain: String,
        axelar: &AxelarChain,
        solana_rpc: String,
    ) -> eyre::Result<Self> {
        let path = storage_file_path(solana_chain_name_on_axelar_chain.as_str());
        let span = tracing::info_span!("deployment file storage");
        let _span_guard = span.enter();
        tracing::info!(?path);
        let file_storage = OpenOptions::new().read(true).write(true).open(path)?;

        // read from file or init new
        let solana_root = Self::from_file(file_storage).unwrap_or_else(|err| {
            tracing::warn!(?err, "initiallizing a new solana deployment file");
            let axelar_config =
                AxelarConfiguration::new_from_axelar_chain_deployment(axelar.clone());
            let solana_configuration = SolanaConfiguration::new(
                solana_chain_name_on_axelar_chain,
                &axelar_config,
                solana_rpc,
            );
            Self {
                solana_configuration,
                axelar_configuration: axelar_config,
                voting_verifier: None,
                axelar_gateway: None,
                multisig_prover: None,
                solana_gateway: None,
                solana_memo_program: None,
                evm_deployments: EvmDeployments::default(),
            }
        });
        solana_root.save()?;

        Ok(solana_root)
    }

    fn from_file(mut reader: impl std::io::Read) -> eyre::Result<Self> {
        let mut data = Vec::new();
        let bytes = reader.read_to_end(&mut data)?;
        tracing::info!(?bytes, "read");
        let slice = &mut data[0..bytes];

        let data = simd_json::from_slice::<Self>(slice)?;
        Ok(data)
    }

    pub(crate) fn save(&self) -> eyre::Result<()> {
        tracing::info!("saving the solana deployment info to disk");
        let data = serde_json::to_string_pretty(&self)?;
        std::fs::write(
            storage_file_path(
                self.solana_configuration
                    .chain_name_on_axelar_chain
                    .as_str(),
            ),
            data.as_bytes(),
        )?;
        Ok(())
    }
}

fn storage_file_path(solana_chain_name_on_axelar_chain: &str) -> std::path::PathBuf {
    let filename = storage_filename(solana_chain_name_on_axelar_chain);
    xtask_crate_root_dir().join(filename)
}

fn storage_filename(solana_chain_name_on_axelar_chain: &str) -> String {
    let filename = format!("{solana_chain_name_on_axelar_chain}.json");
    filename
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AxelarConfiguration {
    pub(crate) axelar_chain: AxelarChain,
    pub(crate) axelar_account_prefix: String,
    pub(crate) axelar_base_denom: String,
    pub(crate) multisig_prover_encoder: String,
    pub(crate) service_name: String,
    pub(crate) verifier_key_type: KeyType,
    pub(crate) voting_verifier_block_expiry: u64,
    pub(crate) voting_verifier_majority_threshould: (u64, u64),
    pub(crate) voting_verifier_confirmation_height: u64,
    pub(crate) voting_verifier_msg_id_format: MessageIdFormat,
    pub(crate) verifier_set_diff_threshold: u32,
    pub(crate) gateway_code_id: Option<u64>,
    pub(crate) voting_verifier_code_id: Option<u64>,
    pub(crate) multisig_prover_code_id: Option<u64>,
}

impl AxelarConfiguration {
    fn new_from_axelar_chain_deployment(axelar: AxelarChain) -> Self {
        Self {
            axelar_base_denom: axelar.contracts.rewards.rewards_denom.clone(),
            multisig_prover_encoder: "rkyv".to_string(),
            verifier_key_type: KeyType::Ecdsa,
            axelar_chain: axelar,
            axelar_account_prefix: "axelar".to_string(),
            service_name: "validators".to_string(),
            voting_verifier_majority_threshould: (1, 1),
            voting_verifier_block_expiry: 10,
            voting_verifier_confirmation_height: 1,
            verifier_set_diff_threshold: 1,
            voting_verifier_msg_id_format: MessageIdFormat::Base58SolanaTxSignatureAndEventIndex,
            gateway_code_id: None,
            voting_verifier_code_id: None,
            multisig_prover_code_id: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SolanaConfiguration {
    pub(crate) chain_name_on_axelar_chain: String,
    pub(crate) domain_separator: [u8; 32],
    pub(crate) rpc: String,
    pub(crate) gateway_program_id: String,
}

impl SolanaConfiguration {
    fn new(
        solana_chain_name_on_axelar_chain: String,
        config: &AxelarConfiguration,
        solana_rpc: String,
    ) -> Self {
        let domain_separator = domain_separator(
            &solana_chain_name_on_axelar_chain,
            &config.axelar_chain.contracts.router.address,
        );
        Self {
            chain_name_on_axelar_chain: solana_chain_name_on_axelar_chain,
            domain_separator,
            rpc: solana_rpc,
            gateway_program_id: gmp_gateway::id().to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct DeploymentTracker {
    items: Vec<SolanaDeploymentRoot>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct VotingVerifierDeployment {
    pub(crate) code_id: u64,
    pub(crate) init_params: voting_verifier::msg::InstantiateMsg,
    pub(crate) address: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct AxelarGatewayDeployment {
    pub(crate) code_id: u64,
    pub(crate) address: String,
    pub(crate) init_params: gateway::msg::InstantiateMsg,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct MultisigProverDeployment {
    pub(crate) code_id: u64,
    pub(crate) address: String,
    pub(crate) init_params: multisig_prover_api::InstantiateMsg,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SolanaGatewayDeployment {
    pub(crate) domain_separator: [u8; 32],
    pub(crate) initial_signer_sets: Vec<VerifierSet>,
    pub(crate) minimum_rotation_delay: u64,
    pub(crate) operator: Pubkey,
    pub(crate) previous_signers_retention: [u8; 32],
    pub(crate) program_id: Pubkey,
    pub(crate) config_pda: Pubkey,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SolanaMemoProgram {
    pub(crate) solana_gateway_root_config_pda: Pubkey,
    pub(crate) program_id: Pubkey,
    pub(crate) counter_pda: Pubkey,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct EvmDeployments {
    chains: BTreeMap<String, CustomEvmChainDeployments>,
}

impl EvmDeployments {
    pub(crate) fn get_or_insert_mut(
        &mut self,
        axelar_evm_chain: &super::axelar_deployments::EvmChain,
    ) -> &mut CustomEvmChainDeployments {
        self.chains
            .entry(axelar_evm_chain.id.clone())
            .or_insert_with(|| CustomEvmChainDeployments {
                name: axelar_evm_chain.name.clone(),
                id: axelar_evm_chain.id.clone(),
                axelar_id: axelar_evm_chain.axelar_id.clone(),
                chain_id: axelar_evm_chain.chain_id,
                rpc: axelar_evm_chain.rpc.clone(),
                token_symbol: axelar_evm_chain.token_symbol.clone(),
                confirmations: axelar_evm_chain.confirmations,
                contracts: axelar_evm_chain.contracts.clone(),
                explorer: axelar_evm_chain.explorer.clone(),
                memo_program_address: None,
            })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct CustomEvmChainDeployments {
    pub(crate) name: String,
    pub(crate) id: String,
    pub(crate) axelar_id: String,
    pub(crate) chain_id: u64,
    pub(crate) rpc: String,
    pub(crate) token_symbol: String,
    pub(crate) confirmations: u64,
    pub(crate) contracts: Contracts,
    pub(crate) explorer: Explorer,
    // our custom deployments
    pub(crate) memo_program_address: Option<String>,
}
