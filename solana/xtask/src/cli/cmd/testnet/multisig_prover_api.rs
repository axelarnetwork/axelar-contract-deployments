// NOTE: there are issues with using `multisig-prover` as a dependency (bulid
// breaks). Thats why the types are re-defined here
use axelar_wasm_std::nonempty::Uint64;
use axelar_wasm_std::MajorityThreshold;
use cosmwasm_schema::cw_serde;
use multisig::key::KeyType;
use router_api::{CrossChainId, Message};

#[cw_serde]
pub(crate) enum MultisigProverExecuteMsg {
    ConstructProof { message_ids: Vec<CrossChainId> },
    UpdateVerifierSet,
}

#[cw_serde]
pub(crate) enum QueryMsg {
    GetProof { multisig_session_id: Uint64 },
}

#[cw_serde]
pub(crate) struct GetProofResponse {
    pub(crate) multisig_session_id: Uint64,
    pub(crate) message_ids: Vec<CrossChainId>,
    pub(crate) payload: Payload,
    pub(crate) status: ProofStatus,
}

#[cw_serde]
pub(crate) enum Payload {
    Messages(Vec<Message>),
}

#[cw_serde]
pub(crate) enum ProofStatus {
    Pending,
    Completed { execute_data: String },
}

#[cw_serde]
pub(crate) struct InstantiateMsg {
    pub(crate) admin_address: String,
    pub(crate) governance_address: String,
    pub(crate) gateway_address: String,
    pub(crate) multisig_address: String,
    pub(crate) coordinator_address: String,
    pub(crate) service_registry_address: String,
    pub(crate) voting_verifier_address: String,
    pub(crate) signing_threshold: MajorityThreshold,
    pub(crate) service_name: String,
    pub(crate) chain_name: String,
    pub(crate) verifier_set_diff_threshold: u32,
    pub(crate) encoder: String,
    pub(crate) key_type: KeyType,
    pub(crate) domain_separator: String,
}
